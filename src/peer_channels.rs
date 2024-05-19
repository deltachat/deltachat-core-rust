//! Peer channels for realtime communication in webxdcs.
//!
//! We use Iroh as an ephemeral peer channels provider to create direct communication
//! channels between webxdcs. See [here](https://webxdc.org/docs/spec/joinRealtimeChannel.html) for the webxdc specs.
//!
//! Ephemeral channels should be established lazily, to avoid bootstrapping p2p connectivity
//! when it's not required. Only when a webxdc subscribes to realtime data or when a reatlime message is sent,
//! the p2p machinery should be started.
//!
//! Adding peer channels to webxdc needs upfront negotation of a topic and sharing of public keys so that
//! nodes can connect to each other. The explicit approach is as follows:
//!
//! 1. We introduce a new [GossipTopic](crate::headerdef::HeaderDef::IrohGossipTopic) message header with a random 32-byte TopicId,
//!    securely generated on the initial webxdc sender's device. This message header is encrypted
//!    and sent in the same message as the webxdc application.
//! 2. Whenever `joinRealtimeChannel().setListener()` or `joinRealtimeChannel().send()` is called by the webxdc application,
//!    we start a routine to establish p2p connectivity and join the gossip swarm with Iroh.
//! 3. The first step of this routine is to introduce yourself with a regular message containing the `IrohPublicKey`.
//!    This message contains the users relay-server and public key.
//!    Direct IP address is not included as this information can be persisted by email providers.
//! 4. After the announcement, the sending peer joins the gossip swarm with an empty list of peer IDs (as they don't know anyone yet).
//! 5. Upon receiving an announcement message, other peers store the sender's [NodeAddr] in the database
//!    (scoped per WebXDC app instance/message-id). The other peers can then join the gossip with `joinRealtimeChannel().setListener()`
//!    and `joinRealtimeChannel().send()` just like the other peers.

use anyhow::{anyhow, Context as _, Result};
use email::Header;
use iroh_gossip::net::{Gossip, JoinTopicFut, GOSSIP_ALPN};
use iroh_gossip::proto::{Event as IrohEvent, TopicId};
use iroh_net::relay::{RelayMap, RelayUrl};
use iroh_net::{key::SecretKey, relay::RelayMode, MagicEndpoint};
use iroh_net::{NodeAddr, NodeId};
use std::collections::{BTreeSet, HashMap};
use std::env;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use url::Url;

use crate::chat::send_msg;
use crate::context::Context;
use crate::headerdef::HeaderDef;
use crate::message::{Message, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::EventType;

/// The length of an ed25519 `PublicKey`, in bytes.
const PUBLIC_KEY_LENGTH: usize = 32;
const PUBLIC_KEY_STUB: &[u8] = "static_string".as_bytes();

/// Store iroh peer channels for the context.
#[derive(Debug)]
pub struct Iroh {
    /// [MagicEndpoint] needed for iroh peer channels.
    pub(crate) endpoint: MagicEndpoint,

    /// [Gossip] needed for iroh peer channels.
    pub(crate) gossip: Gossip,

    /// Topics for which an advertisement has already been sent.
    pub(crate) iroh_channels: RwLock<HashMap<TopicId, ChannelState>>,

    /// Currently used Iroh secret key
    pub(crate) secret_key: SecretKey,
}

impl Iroh {
    /// Notify the endpoint that the network has changed.
    pub(crate) async fn network_change(&self) {
        self.endpoint.network_change().await
    }

    /// Join a topic and create the subscriber loop for it.
    ///
    /// If there is no gossip, create it.
    ///
    /// The returned future resolves when the swarm becomes operational.
    async fn join_and_subscribe_gossip(
        &self,
        ctx: &Context,
        msg_id: MsgId,
    ) -> Result<Option<JoinTopicFut>> {
        let topic = get_iroh_topic_for_msg(ctx, msg_id).await?;
        let seq = if let Some(channel_state) = self.iroh_channels.read().await.get(&topic) {
            if channel_state.subscribe_loop.is_some() {
                return Ok(None);
            }
            channel_state.seq_number
        } else {
            0
        };

        let peers = get_iroh_gossip_peers(ctx, msg_id).await?;
        info!(
            ctx,
            "IROH_REALTIME: Joining gossip with peers: {:?}",
            peers.iter().map(|p| p.node_id).collect::<Vec<_>>()
        );

        // Connect to all peers
        for peer in &peers {
            self.endpoint.add_node_addr(peer.clone())?;
        }

        let connect_future = self
            .gossip
            .join(topic, peers.into_iter().map(|addr| addr.node_id).collect())
            .await?;

        let ctx = ctx.clone();
        let gossip = self.gossip.clone();
        let subscribe_loop = tokio::spawn(async move {
            if let Err(e) = subscribe_loop(&ctx, gossip, topic, msg_id).await {
                warn!(ctx, "subscribe_loop failed: {e}")
            }
        });

        self.iroh_channels
            .write()
            .await
            .insert(topic, ChannelState::new(seq, subscribe_loop));

        Ok(Some(connect_future))
    }

    /// Add gossip peers to realtime channel if it is already active.
    pub async fn maybe_add_gossip_peers(&self, topic: TopicId, peers: Vec<NodeAddr>) -> Result<()> {
        if let Some(state) = self.iroh_channels.read().await.get(&topic) {
            if state.subscribe_loop.is_some() {
                for peer in &peers {
                    self.endpoint.add_node_addr(peer.clone())?;
                }
                self.gossip
                    .join(topic, peers.into_iter().map(|peer| peer.node_id).collect())
                    .await?;
            }
        }
        Ok(())
    }

    /// Send realtime data to the gossip swarm.
    pub async fn send_webxdc_realtime_data(
        &self,
        ctx: &Context,
        msg_id: MsgId,
        mut data: Vec<u8>,
    ) -> Result<()> {
        let topic = get_iroh_topic_for_msg(ctx, msg_id).await?;
        self.join_and_subscribe_gossip(ctx, msg_id).await?;

        let seq_num = self.get_and_incr(&topic).await;
        data.extend(seq_num.to_le_bytes());
        data.extend(self.secret_key.public().as_bytes());

        self.gossip.broadcast(topic, data.into()).await?;

        if env::var("REALTIME_DEBUG").is_ok() {
            info!(ctx, "Sent realtime data");
        }

        Ok(())
    }

    async fn get_and_incr(&self, topic: &TopicId) -> i32 {
        let mut seq = 0;
        if let Some(state) = self.iroh_channels.write().await.get_mut(topic) {
            seq = state.seq_number;
            state.seq_number = state.seq_number.wrapping_add(1)
        }
        seq
    }

    /// Get the iroh [NodeAddr] without direct IP addresses.
    pub(crate) async fn get_node_addr(&self) -> Result<NodeAddr> {
        let mut addr = self.endpoint.my_addr().await?;
        addr.info.direct_addresses = BTreeSet::new();
        Ok(addr)
    }

    /// Leave the realtime channel for a given topic.
    pub(crate) async fn leave_realtime(&self, topic: TopicId) -> Result<()> {
        if let Some(channel) = &mut self.iroh_channels.write().await.get_mut(&topic) {
            if let Some(subscribe_loop) = channel.subscribe_loop.take() {
                subscribe_loop.abort();
            }
        }
        self.gossip.quit(topic).await?;
        Ok(())
    }
}

/// Single gossip channel state.
#[derive(Debug)]
pub(crate) struct ChannelState {
    /// Sequence number for the gossip channel.
    seq_number: i32,
    /// The subscribe loop handle.
    subscribe_loop: Option<JoinHandle<()>>,
}

impl ChannelState {
    fn new(seq_number: i32, subscribe_loop: JoinHandle<()>) -> Self {
        Self {
            seq_number,
            subscribe_loop: Some(subscribe_loop),
        }
    }
}

impl Context {
    /// Create magic endpoint and gossip.
    async fn init_peer_channels(&self) -> Result<Iroh> {
        let secret_key: SecretKey = SecretKey::generate();

        let relay_mode = if let Some(relay_url) = self
            .metadata
            .read()
            .await
            .as_ref()
            .and_then(|conf| conf.iroh_relay.clone())
        {
            RelayMode::Custom(RelayMap::from_url(RelayUrl::from(relay_url)))
        } else {
            // FIXME: this should be RelayMode::Disabled instead.
            // Currently using default relays because otherwise Rust tests fail.
            RelayMode::Default
        };

        let endpoint = MagicEndpoint::builder()
            .secret_key(secret_key.clone())
            .alpns(vec![GOSSIP_ALPN.to_vec()])
            .relay_mode(relay_mode)
            .bind(0)
            .await?;

        // create gossip
        let my_addr = endpoint.my_addr().await?;
        let gossip = Gossip::from_endpoint(endpoint.clone(), Default::default(), &my_addr.info);

        // spawn endpoint loop that forwards incoming connections to the gossiper
        let context = self.clone();

        // Shuts down on deltachat shutdown
        tokio::spawn(endpoint_loop(context, endpoint.clone(), gossip.clone()));

        Ok(Iroh {
            endpoint,
            gossip,
            iroh_channels: RwLock::new(HashMap::new()),
            secret_key,
        })
    }

    /// Get or initialize the iroh peer channel.
    pub async fn get_or_try_init_peer_channel(&self) -> Result<&Iroh> {
        let ctx = self.clone();
        self.iroh
            .get_or_try_init(|| async { ctx.init_peer_channels().await })
            .await
    }
}

/// Cache a peers [NodeId] for one topic.
pub(crate) async fn iroh_add_peer_for_topic(
    ctx: &Context,
    msg_id: MsgId,
    topic: TopicId,
    peer: NodeId,
    relay_server: Option<&str>,
) -> Result<()> {
    ctx.sql
        .execute(
            "INSERT OR REPLACE INTO iroh_gossip_peers (msg_id, public_key, topic, relay_server) VALUES (?, ?, ?, ?)",
            (msg_id, peer.as_bytes(), topic.as_bytes(), relay_server),
        )
        .await?;
    Ok(())
}

/// Insert topicId into the database so that we can use it to retrieve the topic.
pub(crate) async fn insert_topic_stub(ctx: &Context, msg_id: MsgId, topic: TopicId) -> Result<()> {
    ctx.sql
        .execute(
            "INSERT OR REPLACE INTO iroh_gossip_peers (msg_id, public_key, topic, relay_server) VALUES (?, ?, ?, ?)",
            (msg_id, PUBLIC_KEY_STUB, topic.as_bytes(), Option::<&str>::None),
        )
        .await?;
    Ok(())
}

/// Get a list of [NodeAddr]s for one webxdc.
async fn get_iroh_gossip_peers(ctx: &Context, msg_id: MsgId) -> Result<Vec<NodeAddr>> {
    ctx.sql
        .query_map(
            "SELECT public_key, relay_server FROM iroh_gossip_peers WHERE msg_id = ? AND public_key != ?",
            (msg_id, PUBLIC_KEY_STUB),
            |row| {
                let key:  Vec<u8> = row.get(0)?;
                let server: Option<String> = row.get(1)?;
                Ok((key, server))
            },
            |g| {
                g.map(|data| {
                    let (key, server) = data?;
                    let server = server.map(|data| Ok::<_, url::ParseError>(RelayUrl::from(Url::parse(&data)?))).transpose()?;
                    let id = NodeId::from_bytes(&key.try_into()
                    .map_err(|_| anyhow!("Can't convert sql data to [u8; 32]"))?)?;
                    Ok::<_, anyhow::Error>(NodeAddr::from_parts(
                        id, server, vec![]
                    ))
                })
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
            },
        )
        .await
}

/// Get the topic for a given [MsgId].
pub(crate) async fn get_iroh_topic_for_msg(ctx: &Context, msg_id: MsgId) -> Result<TopicId> {
    let bytes: Vec<u8> = ctx
        .sql
        .query_get_value(
            "SELECT topic FROM iroh_gossip_peers WHERE msg_id = ? LIMIT 1",
            (msg_id,),
        )
        .await?
        .context("couldn't restore topic from db")?;
    Ok(TopicId::from_bytes(bytes.try_into().unwrap()))
}

/// Send a gossip advertisement to the chat that [MsgId] belongs to.
/// This method should be called from the frontend when `joinRealtimeChannel` is called.
pub async fn send_webxdc_realtime_advertisement(
    ctx: &Context,
    msg_id: MsgId,
) -> Result<Option<JoinTopicFut>> {
    let iroh = ctx.get_or_try_init_peer_channel().await?;
    let conn = iroh.join_and_subscribe_gossip(ctx, msg_id).await?;

    let webxdc = Message::load_from_db(ctx, msg_id).await?;
    let mut msg = Message::new(Viewtype::Text);
    msg.hidden = true;
    msg.param.set_cmd(SystemMessage::IrohNodeAddr);
    msg.in_reply_to = Some(webxdc.rfc724_mid.clone());
    send_msg(ctx, webxdc.chat_id, &mut msg).await?;
    info!(ctx, "IROH_REALTIME: Sent realtime advertisement");
    Ok(conn)
}

/// Send realtime data to the gossip swarm.
pub async fn send_webxdc_realtime_data(ctx: &Context, msg_id: MsgId, data: Vec<u8>) -> Result<()> {
    let iroh = ctx.get_or_try_init_peer_channel().await?;
    iroh.send_webxdc_realtime_data(ctx, msg_id, data).await?;
    Ok(())
}

/// Leave the gossip of the webxdc with given [MsgId].
pub async fn leave_webxdc_realtime(ctx: &Context, msg_id: MsgId) -> Result<()> {
    let iroh = ctx.get_or_try_init_peer_channel().await?;
    iroh.leave_realtime(get_iroh_topic_for_msg(ctx, msg_id).await?)
        .await?;
    info!(ctx, "IROH_REALTIME: Left gossip for message {msg_id}");

    Ok(())
}

pub(crate) fn create_random_topic() -> TopicId {
    TopicId::from_bytes(rand::random())
}

pub(crate) async fn create_iroh_header(
    ctx: &Context,
    topic: TopicId,
    msg_id: MsgId,
) -> Result<Header> {
    insert_topic_stub(ctx, msg_id, topic).await?;
    Ok(Header::new(
        HeaderDef::IrohGossipTopic.get_headername().to_string(),
        topic.to_string(),
    ))
}

async fn endpoint_loop(context: Context, endpoint: MagicEndpoint, gossip: Gossip) {
    while let Some(conn) = endpoint.accept().await {
        info!(context, "IROH_REALTIME: accepting iroh connection");
        let gossip = gossip.clone();
        let context = context.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(&context, conn, gossip).await {
                warn!(context, "IROH_REALTIME: iroh connection error: {err}");
            }
        });
    }
}

async fn handle_connection(
    context: &Context,
    mut conn: iroh_net::magic_endpoint::Connecting,
    gossip: Gossip,
) -> anyhow::Result<()> {
    let alpn = conn.alpn().await?;
    let conn = conn.await?;
    let peer_id = iroh_net::magic_endpoint::get_remote_node_id(&conn)?;

    match alpn.as_bytes() {
        GOSSIP_ALPN => gossip
            .handle_connection(conn)
            .await
            .context(format!("Connection to {peer_id} with ALPN {alpn} failed"))?,
        _ => warn!(
            context,
            "Ignoring connection from {peer_id}: unsupported ALPN protocol"
        ),
    }
    Ok(())
}

async fn subscribe_loop(
    context: &Context,
    gossip: Gossip,
    topic: TopicId,
    msg_id: MsgId,
) -> Result<()> {
    let mut stream = gossip.subscribe(topic).await?;
    loop {
        let event = stream.recv().await?;
        match event {
            IrohEvent::NeighborUp(node) => {
                info!(context, "IROH_REALTIME: NeighborUp: {}", node.to_string());
                iroh_add_peer_for_topic(context, msg_id, topic, node, None).await?;
            }
            IrohEvent::Received(event) => {
                info!(context, "IROH_REALTIME: Received realtime data");
                context.emit_event(EventType::WebxdcRealtimeData {
                    msg_id,
                    data: event
                        .content
                        .get(0..event.content.len() - 4 - PUBLIC_KEY_LENGTH)
                        .context("too few bytes in iroh message")?
                        .into(),
                });
            }
            _ => (),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chat::send_msg,
        message::{Message, Viewtype},
        test_utils::TestContextManager,
        EventType,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_can_communicate() {
        let mut tcm = TestContextManager::new();
        let alice = &mut tcm.alice().await;
        let bob = &mut tcm.bob().await;

        // Alice sends webxdc to bob
        let alice_chat = alice.create_chat(bob).await;
        let mut instance = Message::new(Viewtype::File);
        instance
            .set_file_from_bytes(
                alice,
                "minimal.xdc",
                include_bytes!("../test-data/webxdc/minimal.xdc"),
                None,
            )
            .await
            .unwrap();

        send_msg(alice, alice_chat.id, &mut instance).await.unwrap();
        let alice_webxdc = alice.get_last_msg().await;
        assert_eq!(alice_webxdc.get_viewtype(), Viewtype::Webxdc);

        let webxdc = alice.pop_sent_msg().await;
        let bob_webdxc = bob.recv_msg(&webxdc).await;
        assert_eq!(bob_webdxc.get_viewtype(), Viewtype::Webxdc);

        bob_webdxc.chat_id.accept(bob).await.unwrap();

        // Alice advertises herself.
        send_webxdc_realtime_advertisement(alice, alice_webxdc.id)
            .await
            .unwrap();

        bob.recv_msg_trash(&alice.pop_sent_msg().await).await;
        let bob_iroh = bob.get_or_try_init_peer_channel().await.unwrap();

        // Bob adds alice to gossip peers.
        let members = get_iroh_gossip_peers(bob, bob_webdxc.id)
            .await
            .unwrap()
            .into_iter()
            .map(|addr| addr.node_id)
            .collect::<Vec<_>>();

        let alice_iroh = alice.get_or_try_init_peer_channel().await.unwrap();
        assert_eq!(
            members,
            vec![alice_iroh.get_node_addr().await.unwrap().node_id]
        );

        bob_iroh
            .join_and_subscribe_gossip(bob, bob_webdxc.id)
            .await
            .unwrap()
            .unwrap()
            .await
            .unwrap();

        // Alice sends ephemeral message
        alice_iroh
            .send_webxdc_realtime_data(alice, alice_webxdc.id, "alice -> bob".as_bytes().to_vec())
            .await
            .unwrap();

        loop {
            let event = bob.evtracker.recv().await.unwrap();
            if let EventType::WebxdcRealtimeData { data, .. } = event.typ {
                if data == "alice -> bob".as_bytes() {
                    break;
                } else {
                    panic!(
                        "Unexpected status update: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
            }
        }
        // Bob sends ephemeral message
        bob_iroh
            .send_webxdc_realtime_data(bob, bob_webdxc.id, "bob -> alice".as_bytes().to_vec())
            .await
            .unwrap();

        loop {
            let event = alice.evtracker.recv().await.unwrap();
            if let EventType::WebxdcRealtimeData { data, .. } = event.typ {
                if data == "bob -> alice".as_bytes() {
                    break;
                } else {
                    panic!(
                        "Unexpected status update: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
            }
        }

        // Alice adds bob to gossip peers.
        let members = get_iroh_gossip_peers(alice, alice_webxdc.id)
            .await
            .unwrap()
            .into_iter()
            .map(|addr| addr.node_id)
            .collect::<Vec<_>>();

        assert_eq!(
            members,
            vec![bob_iroh.get_node_addr().await.unwrap().node_id]
        );

        bob_iroh
            .send_webxdc_realtime_data(bob, bob_webdxc.id, "bob -> alice 2".as_bytes().to_vec())
            .await
            .unwrap();

        loop {
            let event = alice.evtracker.recv().await.unwrap();
            if let EventType::WebxdcRealtimeData { data, .. } = event.typ {
                if data == "bob -> alice 2".as_bytes() {
                    break;
                } else {
                    panic!(
                        "Unexpected status update: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_can_reconnect() {
        let mut tcm = TestContextManager::new();
        let alice = &mut tcm.alice().await;
        let bob = &mut tcm.bob().await;

        // Alice sends webxdc to bob
        let alice_chat = alice.create_chat(bob).await;
        let mut instance = Message::new(Viewtype::File);
        instance
            .set_file_from_bytes(
                alice,
                "minimal.xdc",
                include_bytes!("../test-data/webxdc/minimal.xdc"),
                None,
            )
            .await
            .unwrap();

        send_msg(alice, alice_chat.id, &mut instance).await.unwrap();
        let alice_webxdc = alice.get_last_msg().await;
        assert_eq!(alice_webxdc.get_viewtype(), Viewtype::Webxdc);

        let webxdc = alice.pop_sent_msg().await;
        let bob_webdxc = bob.recv_msg(&webxdc).await;
        assert_eq!(bob_webdxc.get_viewtype(), Viewtype::Webxdc);

        bob_webdxc.chat_id.accept(bob).await.unwrap();

        // Alice advertises herself.
        send_webxdc_realtime_advertisement(alice, alice_webxdc.id)
            .await
            .unwrap();

        bob.recv_msg_trash(&alice.pop_sent_msg().await).await;
        let bob_iroh = bob.get_or_try_init_peer_channel().await.unwrap();

        // Bob adds alice to gossip peers.
        let members = get_iroh_gossip_peers(bob, bob_webdxc.id)
            .await
            .unwrap()
            .into_iter()
            .map(|addr| addr.node_id)
            .collect::<Vec<_>>();

        let alice_iroh = alice.get_or_try_init_peer_channel().await.unwrap();
        assert_eq!(
            members,
            vec![alice_iroh.get_node_addr().await.unwrap().node_id]
        );

        bob_iroh
            .join_and_subscribe_gossip(bob, bob_webdxc.id)
            .await
            .unwrap()
            .unwrap()
            .await
            .unwrap();

        // Alice sends ephemeral message
        alice_iroh
            .send_webxdc_realtime_data(alice, alice_webxdc.id, "alice -> bob".as_bytes().to_vec())
            .await
            .unwrap();

        loop {
            let event = bob.evtracker.recv().await.unwrap();
            if let EventType::WebxdcRealtimeData { data, .. } = event.typ {
                if data == "alice -> bob".as_bytes() {
                    break;
                } else {
                    panic!(
                        "Unexpected status update: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
            }
        }

        // TODO: check that seq number is persisted
        leave_webxdc_realtime(bob, bob_webdxc.id).await.unwrap();

        bob_iroh
            .join_and_subscribe_gossip(bob, bob_webdxc.id)
            .await
            .unwrap()
            .unwrap()
            .await
            .unwrap();

        bob_iroh
            .send_webxdc_realtime_data(bob, bob_webdxc.id, "bob -> alice".as_bytes().to_vec())
            .await
            .unwrap();

        loop {
            let event = alice.evtracker.recv().await.unwrap();
            if let EventType::WebxdcRealtimeData { data, .. } = event.typ {
                if data == "bob -> alice".as_bytes() {
                    break;
                } else {
                    panic!(
                        "Unexpected status update: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
            }
        }

        // channel is only used to remeber if an advertisement has been sent
        // bob for example does not change the channels because he never sends an
        // advertisement
        assert_eq!(
            alice.iroh.get().unwrap().iroh_channels.read().await.len(),
            1
        );
        leave_webxdc_realtime(alice, alice_webxdc.id).await.unwrap();
        let topic = get_iroh_topic_for_msg(alice, alice_webxdc.id)
            .await
            .unwrap();
        assert!(if let Some(state) = alice
            .iroh
            .get()
            .unwrap()
            .iroh_channels
            .read()
            .await
            .get(&topic)
        {
            state.subscribe_loop.is_none()
        } else {
            false
        });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parallel_connect() {
        let mut tcm = TestContextManager::new();
        let alice = &mut tcm.alice().await;
        let bob = &mut tcm.bob().await;

        // Alice sends webxdc to bob
        let alice_chat = alice.create_chat(bob).await;
        let mut instance = Message::new(Viewtype::File);
        instance
            .set_file_from_bytes(
                alice,
                "minimal.xdc",
                include_bytes!("../test-data/webxdc/minimal.xdc"),
                None,
            )
            .await
            .unwrap();
        send_msg(alice, alice_chat.id, &mut instance).await.unwrap();
        let alice_webxdc = alice.get_last_msg().await;

        let webxdc = alice.pop_sent_msg().await;
        let bob_webxdc = bob.recv_msg(&webxdc).await;
        assert_eq!(bob_webxdc.get_viewtype(), Viewtype::Webxdc);

        bob_webxdc.chat_id.accept(bob).await.unwrap();

        eprintln!("Sending advertisements");
        // Alice advertises herself.
        let alice_advertisement_future = send_webxdc_realtime_advertisement(alice, alice_webxdc.id)
            .await
            .unwrap()
            .unwrap();
        let alice_advertisement = alice.pop_sent_msg().await;

        send_webxdc_realtime_advertisement(bob, bob_webxdc.id)
            .await
            .unwrap();
        let bob_advertisement = bob.pop_sent_msg().await;

        eprintln!("Receiving advertisements");
        bob.recv_msg_trash(&alice_advertisement).await;
        alice.recv_msg_trash(&bob_advertisement).await;

        eprintln!("Alice waits for connection");
        alice_advertisement_future.await.unwrap();

        // Alice sends ephemeral message
        eprintln!("Sending ephemeral message");
        send_webxdc_realtime_data(alice, alice_webxdc.id, b"alice -> bob".into())
            .await
            .unwrap();

        eprintln!("Waiting for ephemeral message");
        loop {
            let event = bob.evtracker.recv().await.unwrap();
            if let EventType::WebxdcRealtimeData { data, .. } = event.typ {
                if data == b"alice -> bob" {
                    break;
                } else {
                    panic!(
                        "Unexpected status update: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
            }
        }
    }
}
