use crate::net::http;
use anyhow::Result;

/// Manages subscription to Apple Push Notification services.
#[derive(Debug)]
pub(crate) struct PushSubscriber {
    /// Device token.
    device_token: Option<String>,

    /// True if subscribed to heartbeat push notifications.
    subscribed: bool,
}

impl PushSubscriber {
    /// Creates new push notification subscriber.
    pub(crate) fn new() -> Self {
        Self {
            device_token: None,
            subscribed: false,
        }
    }

    /// Sets device token for Apple Push Notification service.
    pub(crate) fn set_notify_token(&mut self, token: &str) {
        self.device_token = Some(token.to_string());
    }

    /// Subscribes to Apple Push Notificaion service with previously set device token.
    pub(crate) async fn subscribe(&mut self) -> Result<()> {
        if self.subscribed {
            return Ok(());
        }

        let Some(ref token) = self.device_token else {
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
            self.subscribed = true;
        }
        Ok(())
    }
}
