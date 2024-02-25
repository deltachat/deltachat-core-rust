use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::context::Context;
use crate::net::http;

/// Manages subscription to Apple Push Notification services.
#[derive(Debug, Clone, Default)]
pub struct PushSubscriber {
    inner: Arc<RwLock<PushSubscriberState>>,
}

impl PushSubscriber {
    /// Creates new push notification subscriber.
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Sets device token for Apple Push Notification service.
    pub(crate) async fn set_notify_token(&mut self, token: &str) {
        self.inner.write().await.device_token = Some(token.to_string());
    }

    /// Subscribes to Apple Push Notificaion service with previously set device token.
    pub(crate) async fn subscribe(&mut self) -> Result<()> {
        let mut state = self.inner.write().await;

        if state.subscribed == NotifyState::Heartbeat {
            return Ok(());
        }

        let Some(ref token) = state.device_token else {
            return Ok(());
        };

        let socks5_config = None;
        let response = http::get_client(socks5_config)?
            .post("https://notifications.delta.chat/register")
            .body(format!("{{\"token\":\"{token}\"}}"))
            .send()
            .await?;

        let response_status = response.status();
        if response_status.is_success() {
            state.subscribed = NotifyState::Heartbeat;
        }
        Ok(())
    }

    pub(crate) async fn notify_state(&self) -> NotifyState {
        self.inner.read().await.subscribed
    }
}

#[derive(Debug, Default)]
pub(crate) struct PushSubscriberState {
    /// Device token.
    device_token: Option<String>,

    /// If subscribed to heartbeat push notifications.
    subscribed: NotifyState,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum NotifyState {
    /// Not subscribed to push notifications.
    #[default]
    NotConnected = 0,

    /// Subscribed to heartbeat push notifications.
    Heartbeat = 1,
}

impl Context {
    /// Returns push notification subscriber state.
    pub async fn notify_state(&self) -> NotifyState {
        self.push_subscriber.notify_state().await
    }
}
