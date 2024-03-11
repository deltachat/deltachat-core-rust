//! Peer channels for webxdc updates using iroh.

use anyhow::{anyhow, Context as _, Result};
use image::EncodableLayout;
use iroh_base::base32;
use iroh_gossip::net::{Gossip, GOSSIP_ALPN};
use iroh_gossip::proto::{Event as IrohEvent, TopicId};
use iroh_net::magic_endpoint::accept_conn;
use iroh_net::NodeId;
use iroh_net::{derp::DerpMode, key::SecretKey, MagicEndpoint};

use crate::config::Config;
use crate::contact::ContactId;
use crate::context::Context;
use crate::message::{Message, MsgId};
use crate::tools::time;
use crate::webxdc::StatusUpdateItem;

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
            .derp_mode(DerpMode::Default)
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
    pub async fn join_and_subscribe_topic(&self, topic: TopicId, msg_id: MsgId) -> Result<()> {
        info!(&self, "Joining topic {}.", topic.to_string());

        let Some(ref gossip) = *self.gossip.lock().await else {
            warn!(
                self,
                "Not joining topic {topic} because there is no gossip."
            );
            return Ok(());
        };

        // restore old peers from db, if any
        let peers = self.get_peers_for_topic(topic).await?;
        if peers.len() == 0 {
            // TODO: When there's no peers we will never be able to join the gossip?
            warn!(self, "joining gossip with zero peers");
        } else {
            info!(self, "joining gossip with peers: {peers:?}");
            info!(
                self,
                "{:?}",
                self.endpoint
                    .lock()
                    .await
                    .as_ref()
                    .unwrap()
                    .my_addr()
                    .await?
            );
        }

        // TODO: add timeout as the returned future might be pending forever
        let connect_future = gossip.join(topic, peers).await?;

        tokio::spawn(connect_future);
        tokio::spawn(subscribe_loop(self.clone(), gossip.clone(), topic, msg_id));

        Ok(())
    }

    /// Get list of [NodeId]s for one topic.
    /// This is used to rejoin a gossip group when reopening the xdc.
    /// Only [NodeId] is needed because the magic endpoint caches region and derp server for [NodeId]s.
    pub async fn get_peers_for_topic(&self, topic: TopicId) -> Result<Vec<NodeId>> {
        self.sql
            .query_map(
                "SELECT public_key FROM iroh_gossip_peers WHERE topic = ?",
                (topic.as_bytes(),),
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

    /// Remove one cached peer from a topic.
    pub async fn delete_peer_for_topic(&self, topic: TopicId, peer: NodeId) -> Result<()> {
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
                context.delete_peer_for_topic(topic, node).await?;
            }
            IrohEvent::Received(event) => {
                info!(context, "Received: {:?}", event);
                let payload = String::from_utf8_lossy(event.content.as_bytes());
                let mut instance = Message::load_from_db(&context, msg_id).await?;
                let update: StatusUpdateItem = serde_json::from_str(&payload)?;
                context
                    .create_status_update_record(
                        &mut instance,
                        update,
                        time(),
                        false,
                        ContactId::SELF,
                    )
                    .await?;
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use std::{os::unix::thread, str::FromStr, time::Duration};

    use tokio::time::timeout;

    use crate::{
        chat::send_msg,
        message::Viewtype,
        test_utils::TestContextManager,
        webxdc::{join_gossip_topic, StatusUpdateSerial},
        EventType,
    };

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_can_connect() {
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

        let alice_instance = alice.get_last_msg().await;
        assert_eq!(alice_instance.get_viewtype(), Viewtype::Webxdc);

        let webxdc = alice.pop_sent_msg().await;
        let bob_webdxc = bob.recv_msg(&webxdc).await;
        bob_webdxc.chat_id.accept(bob).await.unwrap();

        assert_eq!(bob_webdxc.get_viewtype(), Viewtype::Webxdc);

        // Alice sends webxdc update with gossip.
        // This produces an SMTP message that contains the topic and a header with alices' node id
        alice
            .send_webxdc_status_update_struct(
                alice_instance.id,
                StatusUpdateItem {
                    payload: "test".to_string().into(),
                    gossip_topic: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                    ..Default::default()
                },
                "",
            )
            .await
            .unwrap();

        alice.flush_status_updates().await.unwrap();
        bob.recv_msg(&alice.pop_sent_msg().await).await;

        let status = bob
            .get_webxdc_status_updates(bob_webdxc.id, StatusUpdateSerial::new(0))
            .await
            .unwrap();
        let status_update_items: Vec<StatusUpdateItem> = serde_json::from_str(&status).unwrap();
        let topic = status_update_items[0].gossip_topic.as_ref().unwrap();
        assert_eq!(topic, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        let topic_id = TopicId::from_str(&iroh_base::base32::fmt(topic)).unwrap();
        let topics = bob.get_peers_for_topic(topic_id).await.unwrap();
        assert_eq!(
            topics,
            vec![alice.endpoint.lock().await.as_ref().unwrap().node_id()]
        );

        let mut stream = alice
            .ctx
            .gossip
            .lock()
            .await
            .as_ref()
            .unwrap()
            .subscribe(topic_id)
            .await
            .unwrap();

        // Bob joins topic
        join_gossip_topic(bob, bob_webdxc.id, topic).await.unwrap();

        let event = timeout(Duration::from_secs(5), stream.recv())
            .await
            .unwrap()
            .unwrap();
        match event {
            IrohEvent::NeighborUp(node) => {
                assert_eq!(node, bob.endpoint.lock().await.as_ref().unwrap().node_id());
            }
            _ => panic!("Expected NeighborUp event"),
        }

        // Bob sends webxdc update with gossip.
        bob.send_webxdc_status_update_struct(
            bob_webdxc.id,
            StatusUpdateItem {
                payload: "bob -> alice".to_string().into(),
                gossip_topic: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                ..Default::default()
            },
            "",
        )
        .await
        .unwrap();

        alice.evtracker.try_recv().unwrap();
        while let Ok(event) = alice.evtracker.try_recv() {
            if let EventType::WebxdcStatusUpdate {
                msg_id,
                status_update_serial,
            } = event.typ
            {
                let status_update = alice
                    .get_status_update(msg_id, status_update_serial)
                    .await
                    .unwrap();
                let status_update_item: StatusUpdateItem =
                    serde_json::from_str(&status_update).unwrap();
                println!("{:?}", status_update_item.payload.to_string());
                if status_update_item
                    .payload
                    .to_string()
                    .contains("bob -> alice")
                {
                    break;
                }
            }
        }

        // Alice sends webxdc update with gossip.
        alice
            .send_webxdc_status_update_struct(
                bob_webdxc.id,
                StatusUpdateItem {
                    payload: "alice -> bob".to_string().into(),
                    gossip_topic: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                    ..Default::default()
                },
                "",
            )
            .await
            .unwrap();

        while let Ok(event) = bob.evtracker.try_recv() {
            if let EventType::WebxdcStatusUpdate {
                msg_id,
                status_update_serial,
            } = event.typ
            {
                let status_update = alice
                    .get_status_update(msg_id, status_update_serial)
                    .await
                    .unwrap();

                let status_update_item: StatusUpdateItem =
                    serde_json::from_str(&status_update).unwrap();

                if status_update_item
                    .payload
                    .to_string()
                    .contains("alice -> bob")
                {
                    break;
                }
            }
        }
    }
}
