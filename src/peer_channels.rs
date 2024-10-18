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
use futures_lite::StreamExt;
use iroh_gossip::net::{Event, Gossip, GossipEvent, JoinOptions, GOSSIP_ALPN};
use iroh_gossip::proto::TopicId;
use iroh_net::key::{PublicKey, SecretKey};
use iroh_net::relay::{RelayMap, RelayUrl};
use iroh_net::{relay::RelayMode, Endpoint};
use iroh_net::{NodeAddr, NodeId};
use parking_lot::Mutex;
use std::collections::{BTreeSet, HashMap};
use std::env;
use tokio::sync::{oneshot, RwLock};
use tokio::task::JoinHandle;
use url::Url;

use crate::chat::send_msg;
use crate::config::Config;
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
    /// [Endpoint] needed for iroh peer channels.
    pub(crate) endpoint: Endpoint,

    /// [Gossip] needed for iroh peer channels.
    pub(crate) gossip: Gossip,

    /// Sequence numbers for gossip channels.
    pub(crate) sequence_numbers: Mutex<HashMap<TopicId, i32>>,

    /// Topics for which an advertisement has already been sent.
    pub(crate) iroh_channels: RwLock<HashMap<TopicId, ChannelState>>,

    /// Currently used Iroh public key.
    ///
    /// This is attached to every message to work around `iroh_gossip` deduplication.
    pub(crate) public_key: PublicKey,
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
    ) -> Result<Option<oneshot::Receiver<()>>> {
        let topic = get_iroh_topic_for_msg(ctx, msg_id)
            .await?
            .with_context(|| format!("Message {msg_id} has no gossip topic"))?;

        // Take exclusive lock to make sure
        // no other thread can create a second gossip subscription
        // after we check that it does not exist and before we create a new one.
        // Otherwise we would receive every message twice or more times.
        let mut iroh_channels = self.iroh_channels.write().await;

        if iroh_channels.contains_key(&topic) {
            return Ok(None);
        }

        let peers = get_iroh_gossip_peers(ctx, msg_id).await?;
        let node_ids = peers.iter().map(|p| p.node_id).collect::<Vec<_>>();

        info!(
            ctx,
            "IROH_REALTIME: Joining gossip with peers: {:?}", node_ids,
        );

        // Inform iroh of potentially new node addresses
        for node_addr in &peers {
            if !node_addr.info.is_empty() {
                self.endpoint.add_node_addr(node_addr.clone())?;
            }
        }

        let (join_tx, join_rx) = oneshot::channel();

        let (gossip_sender, gossip_receiver) = self
            .gossip
            .join_with_opts(topic, JoinOptions::with_bootstrap(node_ids))
            .split();

        let ctx = ctx.clone();
        let subscribe_loop = tokio::spawn(async move {
            if let Err(e) = subscribe_loop(&ctx, gossip_receiver, topic, msg_id, join_tx).await {
                warn!(ctx, "subscribe_loop failed: {e}")
            }
        });

        iroh_channels.insert(topic, ChannelState::new(subscribe_loop, gossip_sender));

        Ok(Some(join_rx))
    }

    /// Add gossip peers to realtime channel if it is already active.
    pub async fn maybe_add_gossip_peers(&self, topic: TopicId, peers: Vec<NodeAddr>) -> Result<()> {
        if self.iroh_channels.read().await.get(&topic).is_some() {
            for peer in &peers {
                self.endpoint.add_node_addr(peer.clone())?;
            }

            self.gossip
                .join(topic, peers.into_iter().map(|peer| peer.node_id).collect())
                .await?;
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
        let topic = get_iroh_topic_for_msg(ctx, msg_id)
            .await?
            .with_context(|| format!("Message {msg_id} has no gossip topic"))?;
        self.join_and_subscribe_gossip(ctx, msg_id).await?;

        let seq_num = self.get_and_incr(&topic);

        let mut iroh_channels = self.iroh_channels.write().await;
        let state = iroh_channels
            .get_mut(&topic)
            .context("Just created state does not exist")?;
        data.extend(seq_num.to_le_bytes());
        data.extend(self.public_key.as_bytes());

        state.sender.broadcast(data.into()).await?;

        if env::var("REALTIME_DEBUG").is_ok() {
            info!(ctx, "Sent realtime data");
        }

        Ok(())
    }

    fn get_and_incr(&self, topic: &TopicId) -> i32 {
        let mut sequence_numbers = self.sequence_numbers.lock();
        let entry = sequence_numbers.entry(*topic).or_default();
        *entry = entry.wrapping_add(1);
        *entry
    }

    /// Get the iroh [NodeAddr] without direct IP addresses.
    pub(crate) async fn get_node_addr(&self) -> Result<NodeAddr> {
        let mut addr = self.endpoint.node_addr().await?;
        addr.info.direct_addresses = BTreeSet::new();
        Ok(addr)
    }

    /// Leave the realtime channel for a given topic.
    pub(crate) async fn leave_realtime(&self, topic: TopicId) -> Result<()> {
        if let Some(channel) = self.iroh_channels.write().await.remove(&topic) {
            // Dropping the last GossipTopic results in quitting the topic.
            // It is split into GossipReceiver and GossipSender.
            // GossipSender (`channel.sender`) is dropped automatically.

            // Subscribe loop owns GossipReceiver.
            // Aborting it and waiting for it to be dropped
            // drops the receiver.
            channel.subscribe_loop.abort();
            let _ = channel.subscribe_loop.await;
        }
        Ok(())
    }
}

/// Single gossip channel state.
#[derive(Debug)]
pub(crate) struct ChannelState {
    /// The subscribe loop handle.
    subscribe_loop: JoinHandle<()>,

    sender: iroh_gossip::net::GossipSender,
}

impl ChannelState {
    fn new(subscribe_loop: JoinHandle<()>, sender: iroh_gossip::net::GossipSender) -> Self {
        Self {
            subscribe_loop,
            sender,
        }
    }
}

impl Context {
    /// Create iroh endpoint and gossip.
    async fn init_peer_channels(&self) -> Result<Iroh> {
        let secret_key = SecretKey::generate();
        let public_key = secret_key.public();

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

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![GOSSIP_ALPN.to_vec()])
            .relay_mode(relay_mode)
            .bind()
            .await?;

        // create gossip
        let my_addr = endpoint.node_addr().await?;
        let gossip = Gossip::from_endpoint(endpoint.clone(), Default::default(), &my_addr.info);

        // spawn endpoint loop that forwards incoming connections to the gossiper
        let context = self.clone();

        // Shuts down on deltachat shutdown
        tokio::spawn(endpoint_loop(context, endpoint.clone(), gossip.clone()));

        Ok(Iroh {
            endpoint,
            gossip,
            sequence_numbers: Mutex::new(HashMap::new()),
            iroh_channels: RwLock::new(HashMap::new()),
            public_key,
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
pub(crate) async fn get_iroh_topic_for_msg(
    ctx: &Context,
    msg_id: MsgId,
) -> Result<Option<TopicId>> {
    if let Some(bytes) = ctx
        .sql
        .query_get_value::<Vec<u8>>(
            "SELECT topic FROM iroh_gossip_peers WHERE msg_id = ? LIMIT 1",
            (msg_id,),
        )
        .await
        .context("Couldn't restore topic from db")?
    {
        let topic_id = TopicId::from_bytes(
            bytes
                .try_into()
                .map_err(|_| anyhow!("Could not convert stored topic ID"))?,
        );
        Ok(Some(topic_id))
    } else {
        Ok(None)
    }
}

/// Send a gossip advertisement to the chat that [MsgId] belongs to.
/// This method should be called from the frontend when `joinRealtimeChannel` is called.
pub async fn send_webxdc_realtime_advertisement(
    ctx: &Context,
    msg_id: MsgId,
) -> Result<Option<oneshot::Receiver<()>>> {
    if !ctx.get_config_bool(Config::WebxdcRealtimeEnabled).await? {
        return Ok(None);
    }

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

/// Send realtime data to other peers using iroh.
pub async fn send_webxdc_realtime_data(ctx: &Context, msg_id: MsgId, data: Vec<u8>) -> Result<()> {
    if !ctx.get_config_bool(Config::WebxdcRealtimeEnabled).await? {
        return Ok(());
    }

    let iroh = ctx.get_or_try_init_peer_channel().await?;
    iroh.send_webxdc_realtime_data(ctx, msg_id, data).await?;
    Ok(())
}

/// Leave the gossip of the webxdc with given [MsgId].
pub async fn leave_webxdc_realtime(ctx: &Context, msg_id: MsgId) -> Result<()> {
    if !ctx.get_config_bool(Config::WebxdcRealtimeEnabled).await? {
        return Ok(());
    }
    let topic = get_iroh_topic_for_msg(ctx, msg_id)
        .await?
        .with_context(|| format!("Message {msg_id} has no gossip topic"))?;
    let iroh = ctx.get_or_try_init_peer_channel().await?;
    iroh.leave_realtime(topic).await?;
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

async fn endpoint_loop(context: Context, endpoint: Endpoint, gossip: Gossip) {
    while let Some(conn) = endpoint.accept().await {
        let conn = match conn.accept() {
            Ok(conn) => conn,
            Err(err) => {
                warn!(context, "Failed to accept iroh connection: {err:#}.");
                continue;
            }
        };
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
    mut conn: iroh_net::endpoint::Connecting,
    gossip: Gossip,
) -> anyhow::Result<()> {
    let alpn = conn.alpn().await?;
    let conn = conn.await?;
    let peer_id = iroh_net::endpoint::get_remote_node_id(&conn)?;

    match alpn.as_slice() {
        GOSSIP_ALPN => gossip
            .handle_connection(conn)
            .await
            .context(format!("Gossip connection to {peer_id} failed"))?,
        _ => warn!(
            context,
            "Ignoring connection from {peer_id}: unsupported ALPN protocol"
        ),
    }
    Ok(())
}

async fn subscribe_loop(
    context: &Context,
    mut stream: iroh_gossip::net::GossipReceiver,
    topic: TopicId,
    msg_id: MsgId,
    join_tx: oneshot::Sender<()>,
) -> Result<()> {
    let mut join_tx = Some(join_tx);

    while let Some(event) = stream.try_next().await? {
        match event {
            Event::Gossip(event) => match event {
                GossipEvent::Joined(nodes) => {
                    if let Some(join_tx) = join_tx.take() {
                        // Try to notify that at least one peer joined,
                        // but ignore the error if receiver is dropped and nobody listens.
                        join_tx.send(()).ok();
                    }

                    for node in nodes {
                        iroh_add_peer_for_topic(context, msg_id, topic, node, None).await?;
                    }
                }
                GossipEvent::NeighborUp(node) => {
                    info!(context, "IROH_REALTIME: NeighborUp: {}", node.to_string());
                    iroh_add_peer_for_topic(context, msg_id, topic, node, None).await?;
                }
                GossipEvent::NeighborDown(_node) => {}
                GossipEvent::Received(message) => {
                    info!(context, "IROH_REALTIME: Received realtime data");
                    context.emit_event(EventType::WebxdcRealtimeData {
                        msg_id,
                        data: message
                            .content
                            .get(0..message.content.len() - 4 - PUBLIC_KEY_LENGTH)
                            .context("too few bytes in iroh message")?
                            .into(),
                    });
                }
            },
            Event::Lagged => {
                warn!(context, "Gossip lost some messages");
            }
        };
    }
    Ok(())
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

        bob.ctx
            .set_config_bool(Config::WebxdcRealtimeEnabled, true)
            .await
            .unwrap();

        alice
            .ctx
            .set_config_bool(Config::WebxdcRealtimeEnabled, true)
            .await
            .unwrap();

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
        let bob_webxdc = bob.recv_msg(&webxdc).await;
        assert_eq!(bob_webxdc.get_viewtype(), Viewtype::Webxdc);

        bob_webxdc.chat_id.accept(bob).await.unwrap();

        // Alice advertises herself.
        send_webxdc_realtime_advertisement(alice, alice_webxdc.id)
            .await
            .unwrap();

        bob.recv_msg_trash(&alice.pop_sent_msg().await).await;
        loop {
            let event = bob.evtracker.recv().await.unwrap();
            if let EventType::WebxdcRealtimeAdvertisementReceived { msg_id } = event.typ {
                assert!(msg_id == alice_webxdc.id);
                break;
            }
        }
        let bob_iroh = bob.get_or_try_init_peer_channel().await.unwrap();

        // Bob adds alice to gossip peers.
        let members = get_iroh_gossip_peers(bob, bob_webxdc.id)
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
            .join_and_subscribe_gossip(bob, bob_webxdc.id)
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
            .send_webxdc_realtime_data(bob, bob_webxdc.id, "bob -> alice".as_bytes().to_vec())
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
            .send_webxdc_realtime_data(bob, bob_webxdc.id, "bob -> alice 2".as_bytes().to_vec())
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

        bob.ctx
            .set_config_bool(Config::WebxdcRealtimeEnabled, true)
            .await
            .unwrap();

        alice
            .ctx
            .set_config_bool(Config::WebxdcRealtimeEnabled, true)
            .await
            .unwrap();

        assert!(alice
            .get_config_bool(Config::WebxdcRealtimeEnabled)
            .await
            .unwrap());
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
        let bob_webxdc = bob.recv_msg(&webxdc).await;
        assert_eq!(bob_webxdc.get_viewtype(), Viewtype::Webxdc);

        bob_webxdc.chat_id.accept(bob).await.unwrap();

        // Alice advertises herself.
        send_webxdc_realtime_advertisement(alice, alice_webxdc.id)
            .await
            .unwrap();

        bob.recv_msg_trash(&alice.pop_sent_msg().await).await;
        let bob_iroh = bob.get_or_try_init_peer_channel().await.unwrap();

        // Bob adds alice to gossip peers.
        let members = get_iroh_gossip_peers(bob, bob_webxdc.id)
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
            .join_and_subscribe_gossip(bob, bob_webxdc.id)
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

        let bob_topic = get_iroh_topic_for_msg(bob, bob_webxdc.id)
            .await
            .unwrap()
            .unwrap();
        let bob_sequence_number = bob
            .iroh
            .get()
            .unwrap()
            .sequence_numbers
            .lock()
            .get(&bob_topic)
            .copied();
        leave_webxdc_realtime(bob, bob_webxdc.id).await.unwrap();
        let bob_sequence_number_after = bob
            .iroh
            .get()
            .unwrap()
            .sequence_numbers
            .lock()
            .get(&bob_topic)
            .copied();
        // Check that sequence number is persisted when leaving the channel.
        assert_eq!(bob_sequence_number, bob_sequence_number_after);

        bob_iroh
            .join_and_subscribe_gossip(bob, bob_webxdc.id)
            .await
            .unwrap()
            .unwrap()
            .await
            .unwrap();

        bob_iroh
            .send_webxdc_realtime_data(bob, bob_webxdc.id, "bob -> alice".as_bytes().to_vec())
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
            .unwrap()
            .unwrap();
        assert!(alice
            .iroh
            .get()
            .unwrap()
            .iroh_channels
            .read()
            .await
            .get(&topic)
            .is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parallel_connect() {
        let mut tcm = TestContextManager::new();
        let alice = &mut tcm.alice().await;
        let bob = &mut tcm.bob().await;

        bob.ctx
            .set_config_bool(Config::WebxdcRealtimeEnabled, true)
            .await
            .unwrap();

        alice
            .ctx
            .set_config_bool(Config::WebxdcRealtimeEnabled, true)
            .await
            .unwrap();

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_peer_channels_disabled() {
        let mut tcm = TestContextManager::new();
        let alice = &mut tcm.alice().await;

        // creates iroh endpoint as side effect
        send_webxdc_realtime_advertisement(alice, MsgId::new(1))
            .await
            .unwrap();

        assert!(alice.ctx.iroh.get().is_none());

        // creates iroh endpoint as side effect
        send_webxdc_realtime_data(alice, MsgId::new(1), vec![])
            .await
            .unwrap();

        assert!(alice.ctx.iroh.get().is_none());

        // creates iroh endpoint as side effect
        leave_webxdc_realtime(alice, MsgId::new(1)).await.unwrap();

        assert!(alice.ctx.iroh.get().is_none())
    }
}
