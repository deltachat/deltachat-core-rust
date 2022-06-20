//! # SMTP message sending

use super::Smtp;
use async_smtp::{EmailAddress, Envelope, SendableEmail, Transport};

use crate::config::Config;
use crate::constants::DEFAULT_MAX_SMTP_RCPT_TO;
use crate::context::Context;
use crate::events::EventType;
use std::time::Duration;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Envelope error: {}", _0)]
    Envelope(#[from] async_smtp::error::Error),
    #[error("Send error: {}", _0)]
    SmtpSend(#[from] async_smtp::smtp::error::Error),
    #[error("SMTP has no transport")]
    NoTransport,
    #[error("{}", _0)]
    Other(#[from] anyhow::Error),
}

impl Smtp {
    /// Send a prepared mail to recipients.
    /// On successful send out Ok() is returned.
    pub async fn send(
        &mut self,
        context: &Context,
        recipients: &[EmailAddress],
        message: &[u8],
        rowid: i64,
    ) -> Result<()> {
        if !context.get_config_bool(Config::Bot).await? {
            // Notify ratelimiter about sent message regardless of whether quota is exceeded or not.
            // Checking whether sending is allowed for low-priority messages should be done by the
            // caller.
            context.ratelimit.write().await.send();
        }

        let message_len_bytes = message.len();

        let mut chunk_size = DEFAULT_MAX_SMTP_RCPT_TO;
        if let Some(provider) = context.get_configured_provider().await? {
            if let Some(max_smtp_rcpt_to) = provider.max_smtp_rcpt_to {
                chunk_size = max_smtp_rcpt_to as usize;
            }
        }

        for recipients_chunk in recipients.chunks(chunk_size) {
            let recipients_display = recipients_chunk
                .iter()
                .map(|x| x.as_ref())
                .collect::<Vec<&str>>()
                .join(",");

            let envelope = Envelope::new(self.from.clone(), recipients_chunk.to_vec())
                .map_err(Error::Envelope)?;
            let mail = SendableEmail::new(
                envelope,
                rowid.to_string(), // only used for internal logging
                message,
            );

            if let Some(ref mut transport) = self.transport {
                // The timeout is 1min + 3min per MB.
                let timeout = 60 + (180 * message_len_bytes / 1_000_000) as u64;
                transport
                    .send_with_timeout(mail, Some(&Duration::from_secs(timeout)))
                    .await
                    .map_err(Error::SmtpSend)?;

                context.emit_event(EventType::SmtpMessageSent(format!(
                    "Message len={} was smtp-sent to {}",
                    message_len_bytes, recipients_display
                )));
                self.last_success = Some(std::time::SystemTime::now());
            } else {
                warn!(
                    context,
                    "uh? SMTP has no transport, failed to send to {}", recipients_display
                );
                return Err(Error::NoTransport);
            }
        }
        Ok(())
    }
}
