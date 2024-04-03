//! Peer channels for webxdc updates using iroh.

use anyhow::{anyhow, Context as _, Result};
use iroh_base::base32;
use iroh_gossip::net::{Gossip, GOSSIP_ALPN};
use iroh_gossip::proto::{Event as IrohEvent, TopicId};
use iroh_net::magic_endpoint::accept_conn;
use iroh_net::{key::SecretKey, relay::RelayMode, MagicEndpoint};
use iroh_net::{NodeAddr, NodeId};
use serde::{Deserialize, Serialize};

use crate::chat::send_msg;
use crate::config::Config;
use crate::context::Context;
use crate::message::{Message, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::EventType;

impl Context {
    /// Create magic endpoint and gossip for the context.
    pub async fn create_gossip(&self) -> Result<()> {
        let secret_key: SecretKey = self.get_or_generate_iroh_keypair().await?;
        println!("> our secret key: {}", base32::fmt(secret_key.to_bytes()));

        if self.endpoint.lock().await.is_some() {
            warn!(
                self,
                "Tried to create endpoint even though there is already one."
            );
            return Ok(());
        }

        // build magic endpoint
        let endpoint = MagicEndpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![GOSSIP_ALPN.to_vec()])
            .relay_mode(RelayMode::Default)
            .bind(0)
            .await?;

        // create gossip
        let my_addr = endpoint.my_addr().await?;
        let gossip = Gossip::from_endpoint(endpoint.clone(), Default::default(), &my_addr.info);

        // spawn endpoint loop that forwards incoming connections to the gossiper
        let context = self.clone();
        tokio::spawn(endpoint_loop(context, endpoint.clone(), gossip.clone()));

        *self.gossip.lock().await = Some(gossip);
        *self.endpoint.lock().await = Some(endpoint);
        Ok(())
    }

    /// Join a topic and create the subscriber loop for it.
    pub async fn join_and_subscribe_gossip(&self, msg_id: MsgId) -> Result<()> {
        let Some(ref gossip) = *self.gossip.lock().await else {
            warn!(
                self,
                "Not joining topic {msg_id} because there is no gossip."
            );
            return Ok(());
        };

        let peers = self.get_gossip_peers(msg_id).await?;
        if peers.is_empty() {
            warn!(self, "joining gossip with zero peers");
        }

        let topic = self.get_topic_for_msg_id(msg_id).await?;
        let connect_future = gossip.join(topic, peers).await?;

        tokio::spawn(connect_future);
        tokio::spawn(subscribe_loop(self.clone(), gossip.clone(), topic, msg_id));

        Ok(())
    }

    /// Cache a peers [NodeId] for one topic.
    pub async fn add_peer_for_topic(
        &self,
        msg_id: MsgId,
        topic: TopicId,
        peer: NodeId,
    ) -> Result<()> {
        self.sql
            .execute(
                "INSERT INTO iroh_gossip_peers (msg_id, public_key, topic) VALUES (?, ?, ?)",
                (msg_id, peer.as_bytes(), topic.as_bytes()),
            )
            .await?;
        Ok(())
    }

    /// Get list of [NodeId]s for one webxdc.
    pub async fn get_gossip_peers(&self, msg_id: MsgId) -> Result<Vec<NodeId>> {
        self.sql
            .query_map(
                "SELECT public_key FROM iroh_gossip_peers WHERE msg_id = ? AND public_key != ?",
                (msg_id, self.get_iroh_node_addr().await?.node_id.as_bytes()),
                |row| {
                    let data = row.get::<_, Vec<u8>>(0)?;
                    Ok(data)
                },
                |g| {
                    g.map(|data| {
                        Ok::<NodeId, anyhow::Error>(NodeId::from_bytes(
                            &data?
                                .try_into()
                                .map_err(|_| anyhow!("Can't convert sql data to [u8; 32]"))?,
                        )?)
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
                },
            )
            .await
    }

    /// Remove one cached peer from a topic.
    pub async fn delete_webxdc_gossip_peer_for_msg(
        &self,
        topic: TopicId,
        peer: NodeId,
    ) -> Result<()> {
        self.sql
            .execute(
                "DELETE FROM iroh_gossip_peers WHERE public_key = ? topic = ?",
                (peer.as_bytes(), topic.as_bytes()),
            )
            .await?;
        Ok(())
    }

    /// Get the iroh gossip secret key from the database or generate a new one and persist it.
    pub async fn get_or_generate_iroh_keypair(&self) -> Result<SecretKey> {
        match self.get_config_parsed(Config::IrohSecretKey).await? {
            Some(key) => Ok(key),
            None => {
                let key = SecretKey::generate();
                self.set_config(Config::IrohSecretKey, Some(&key.to_string()))
                    .await?;
                Ok(key)
            }
        }
    }

    /// Get own iroh gossip public key.
    /// TODO: cache?
    pub async fn get_iroh_node_addr(&self) -> Result<NodeAddr> {
        self.endpoint.lock().await.as_ref().unwrap().my_addr().await
    }

    /// Get the topic for given [MsgId].
    pub async fn get_topic_for_msg_id(&self, msg_id: MsgId) -> Result<TopicId> {
        let bytes = self
            .sql
            .query_row(
                "SELECT topic FROM iroh_gossip_peers WHERE msg_id = ? AND public_key = ?",
                (msg_id, self.get_iroh_node_addr().await?.node_id.as_bytes()),
                |row| {
                    let data = row.get::<_, Vec<u8>>(0)?;
                    Ok(data)
                },
            )
            .await?;
        Ok(TopicId::from_bytes(bytes.try_into().unwrap()))
    }

    /// Send a webxdc ephemeral status update to the gossip network.
    pub async fn send_webxdc_ephemeral_status_update(
        &self,
        msg_id: MsgId,
        status_update: &str,
    ) -> Result<()> {
        let topic = self.get_topic_for_msg_id(msg_id).await.unwrap();
        let message = GossipMessage {
            payload: status_update.to_string(),
            time: chrono::Utc::now().timestamp() as u64,
        };
        self.gossip
            .lock()
            .await
            .as_ref()
            .context("No gossip")?
            .broadcast(topic, serde_json::to_vec(&message)?.into())
            .await?;
        Ok(())
    }

    /// Send a gossip advertisement to the chat of a given [MsgId].
    pub async fn send_gossip_advertisement(&self, msg_id: MsgId) -> Result<()> {
        self.join_and_subscribe_gossip(msg_id).await?;
        let mut msg = Message::new(Viewtype::Text);
        let webxdc = Message::load_from_db(self, msg_id).await?;
        msg.param.set_cmd(SystemMessage::IrohGossipAdvertisement);
        msg.in_reply_to = Some(webxdc.rfc724_mid.clone());
        send_msg(self, webxdc.chat_id, &mut msg).await?;
        Ok(())
    }
}

// Maybe we can add the timstamp in the byte sequence of the message?
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GossipMessage {
    payload: String,
    time: u64,
}

pub(crate) fn create_random_topic() -> TopicId {
    TopicId::from_bytes(rand::random())
}

async fn endpoint_loop(context: Context, endpoint: MagicEndpoint, gossip: Gossip) {
    while let Some(conn) = endpoint.accept().await {
        info!(context, "accepting connection with {:?}", conn);
        let gossip = gossip.clone();
        let context = context.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(&context, conn, gossip).await {
                warn!(context, "iroh connection error: {err}");
            }
        });
    }
}

async fn handle_connection(
    context: &Context,
    conn: quinn::Connecting,
    gossip: Gossip,
) -> anyhow::Result<()> {
    let (peer_id, alpn, conn) = accept_conn(conn).await?;
    match alpn.as_bytes() {
        GOSSIP_ALPN => gossip
            .handle_connection(conn)
            .await
            .context(format!("Connection to {peer_id} with ALPN {alpn} failed"))?,
        _ => info!(
            context,
            "Ignoring connection from {peer_id}: unsupported ALPN protocol"
        ),
    }
    Ok(())
}

async fn subscribe_loop(
    context: Context,
    gossip: Gossip,
    topic: TopicId,
    msg_id: MsgId,
) -> Result<()> {
    let mut stream = gossip.subscribe(topic).await?;
    loop {
        let event = stream.recv().await?;
        match event {
            IrohEvent::NeighborUp(node) => {
                info!(context, "NeighborUp: {:?}", node);
                context.add_peer_for_topic(msg_id, topic, node).await?;
            }
            IrohEvent::NeighborDown(node) => {
                info!(context, "NeighborDown: {:?}", node);
                context
                    .delete_webxdc_gossip_peer_for_msg(topic, node)
                    .await?;
            }
            IrohEvent::Received(event) => {
                info!(context, "Received: {:?}", event);
                let status_update: GossipMessage = serde_json::from_slice(&event.content)?;
                context.emit_event(EventType::WebxdcEphemeralStatusUpdate {
                    msg_id,
                    status_update: status_update.payload,
                });
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
        alice.ctx.start_io().await;
        bob.ctx.start_io().await;

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
        alice
            .send_gossip_advertisement(alice_webxdc.id)
            .await
            .unwrap();

        bob.recv_msg(&alice.pop_sent_msg().await).await;

        // Bob adds alice to gossip peers.
        let members = bob.get_gossip_peers(bob_webdxc.id).await.unwrap();
        assert_eq!(
            members,
            vec![alice.get_iroh_node_addr().await.unwrap().node_id]
        );

        bob.join_and_subscribe_gossip(bob_webdxc.id).await.unwrap();

        tokio::time::sleep(Duration::from_millis(1000)).await;
        // Alice sends ephemeral message
        alice
            .send_webxdc_ephemeral_status_update(alice_webxdc.id, "alice -> bob")
            .await
            .unwrap();

        loop {
            let event = bob.evtracker.recv().await.unwrap();
            if let EventType::WebxdcEphemeralStatusUpdate { status_update, .. } = event.typ {
                if status_update.contains("alice -> bob") {
                    break;
                } else {
                    panic!("Unexpected status update: {status_update}");
                }
            }
        }

        // Bob sends ephemeral message
        bob.send_webxdc_ephemeral_status_update(bob_webdxc.id, "bob -> alice")
            .await
            .unwrap();

        loop {
            let event = alice.evtracker.recv().await.unwrap();
            if let EventType::WebxdcEphemeralStatusUpdate { status_update, .. } = event.typ {
                if status_update.contains("bob -> alice") {
                    break;
                } else {
                    panic!("Unexpected status update: {status_update}");
                }
            }
        }

        // Alice adds bob to gossip peers.
        let members = alice.get_gossip_peers(alice_webxdc.id).await.unwrap();
        assert_eq!(
            members,
            vec![bob.get_iroh_node_addr().await.unwrap().node_id]
        );

        bob.send_webxdc_ephemeral_status_update(bob_webdxc.id, "bob -> alice 2")
            .await
            .unwrap();

        loop {
            let event = alice.evtracker.recv().await.unwrap();
            if let EventType::WebxdcEphemeralStatusUpdate { status_update, .. } = event.typ {
                if status_update.contains("bob -> alice 2") {
                    break;
                } else {
                    panic!("Unexpected status update: {status_update}");
                }
            }
        }
    }
}

