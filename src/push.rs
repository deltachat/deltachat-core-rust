use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::context::Context;

/// Manages subscription to Apple Push Notification services.
///
/// This structure is created by account manager and is shared between accounts.
/// To enable notifications, application should request the device token as described in
/// <https://developer.apple.com/documentation/usernotifications/registering-your-app-with-apns>
/// and give it to the account manager, which will forward the token in this structure.
///
/// Each account (context) can then retrieve device token
/// from this structure and give it to the email server.
/// If email server does not support push notifications,
/// account can call `subscribe` method
/// to register device token with the heartbeat
/// notification provider server as a fallback.
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
    pub(crate) async fn set_device_token(&self, token: &str) {
        self.inner.write().await.device_token = Some(token.to_string());
    }

    /// Retrieves device token.
    ///
    /// Token may be not available if application is not running on Apple platform,
    /// failed to register for remote notifications or is in the process of registering.
    ///
    /// IMAP loop should periodically check if device token is available
    /// and send the token to the email server if it supports push notifications.
    pub(crate) async fn device_token(&self) -> Option<String> {
        self.inner.read().await.device_token.clone()
    }

    /// Subscribes for heartbeat notifications with previously set device token.
    #[cfg(target_os = "ios")]
    pub(crate) async fn subscribe(&self, context: &Context) -> Result<()> {
        use crate::net::http;

        let mut state = self.inner.write().await;

        if state.heartbeat_subscribed {
            return Ok(());
        }

        let Some(ref token) = state.device_token else {
            return Ok(());
        };

        if http::post_string(
            context,
            "https://notifications.delta.chat/register",
            format!("{{\"token\":\"{token}\"}}"),
        )
        .await?
        {
            state.heartbeat_subscribed = true;
        }
        Ok(())
    }

    /// Placeholder to skip subscribing to heartbeat notifications outside iOS.
    #[cfg(not(target_os = "ios"))]
    pub(crate) async fn subscribe(&self, _context: &Context) -> Result<()> {
        let mut state = self.inner.write().await;
        state.heartbeat_subscribed = true;
        Ok(())
    }

    pub(crate) async fn heartbeat_subscribed(&self) -> bool {
        self.inner.read().await.heartbeat_subscribed
    }
}

#[derive(Debug, Default)]
pub(crate) struct PushSubscriberState {
    /// Device token.
    device_token: Option<String>,

    /// If subscribed to heartbeat push notifications.
    heartbeat_subscribed: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum NotifyState {
    /// Not subscribed to push notifications.
    #[default]
    NotConnected = 0,

    /// Subscribed to heartbeat push notifications.
    Heartbeat = 1,

    /// Subscribed to push notifications for new messages.
    Connected = 2,
}

impl Context {
    /// Returns push notification subscriber state.
    pub async fn push_state(&self) -> NotifyState {
        if self.push_subscribed.load(Ordering::Relaxed) {
            NotifyState::Connected
        } else if self.push_subscriber.heartbeat_subscribed().await {
            NotifyState::Heartbeat
        } else {
            NotifyState::NotConnected
        }
    }
}
