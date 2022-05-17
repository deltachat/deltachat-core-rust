//! # Ephemeral messages.
//!
//! Ephemeral messages are messages that have an Ephemeral-Timer
//! header attached to them, which specifies time in seconds after
//! which the message should be deleted both from the device and from
//! the server. The timer is started when the message is marked as
//! seen, which usually happens when its contents is displayed on
//! device screen.
//!
//! Each chat, including 1:1, group chats and "saved messages" chat,
//! has its own ephemeral timer setting, which is applied to all
//! messages sent to the chat. The setting is synchronized to all the
//! devices participating in the chat by applying the timer value from
//! all received messages, including BCC-self ones, to the chat. This
//! way the setting is eventually synchronized among all participants.
//!
//! When user changes ephemeral timer setting for the chat, a system
//! message is automatically sent to update the setting for all
//! participants. This allows changing the setting for a chat like any
//! group chat setting, e.g. name and avatar, without the need to
//! write an actual message.
//!
//! ## Device settings
//!
//! In addition to per-chat ephemeral message setting, each device has
//! two global user-configured settings that complement per-chat
//! settings: `delete_device_after` and `delete_server_after`. These
//! settings are not synchronized among devices and apply to all
//! messages known to the device, including messages sent or received
//! before configuring the setting.
//!
//! `delete_device_after` configures the maximum time device is
//! storing the messages locally. `delete_server_after` configures the
//! time after which device will delete the messages it knows about
//! from the server.
//!
//! ## How messages are deleted
//!
//! When the message is deleted locally, its contents is removed and
//! it is moved to the trash chat. This database entry is then used to
//! track the Message-ID and corresponding IMAP folder and UID until
//! the message is deleted from the server. Vice versa, when device
//! deletes the message from the server, it removes IMAP folder and
//! UID information, but keeps the message contents. When database
//! entry is both moved to trash chat and does not contain UID
//! information, it is deleted from the database, leaving no trace of
//! the message.
//!
//! ## When messages are deleted
//!
//! The `ephemeral_loop` task schedules the next due running of
//! `delete_expired_messages` which in turn emits `MsgsChanged` events
//! when deleting local messages to make UIs reload displayed messages.
//!
//! Server deletion happens by updating the `imap` table based on
//! the database entries which are expired either according to their
//! ephemeral message timers or global `delete_server_after` setting.

use std::convert::{TryFrom, TryInto};
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{ensure, Context as _, Result};
use async_std::channel::Receiver;
use async_std::future::timeout;
use serde::{Deserialize, Serialize};

use crate::chat::{send_msg, ChatId};
use crate::constants::{DC_CHAT_ID_LAST_SPECIAL, DC_CHAT_ID_TRASH};
use crate::contact::ContactId;
use crate::context::Context;
use crate::dc_tools::{duration_to_str, time};
use crate::download::MIN_DELETE_SERVER_AFTER;
use crate::events::EventType;
use crate::log::LogExt;
use crate::message::{Message, MessageState, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::sql::{self, params_iter};
use crate::stock_str;
use std::cmp::max;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum Timer {
    Disabled,
    Enabled { duration: u32 },
}

impl Timer {
    pub fn to_u32(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::Enabled { duration } => duration,
        }
    }

    pub fn from_u32(duration: u32) -> Self {
        if duration == 0 {
            Self::Disabled
        } else {
            Self::Enabled { duration }
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::Disabled
    }
}

impl ToString for Timer {
    fn to_string(&self) -> String {
        self.to_u32().to_string()
    }
}

impl FromStr for Timer {
    type Err = ParseIntError;

    fn from_str(input: &str) -> Result<Timer, ParseIntError> {
        input.parse::<u32>().map(Self::from_u32)
    }
}

impl rusqlite::types::ToSql for Timer {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let val = rusqlite::types::Value::Integer(match self {
            Self::Disabled => 0,
            Self::Enabled { duration } => i64::from(*duration),
        });
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

impl rusqlite::types::FromSql for Timer {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).and_then(|value| {
            if value == 0 {
                Ok(Self::Disabled)
            } else if let Ok(duration) = u32::try_from(value) {
                Ok(Self::Enabled { duration })
            } else {
                Err(rusqlite::types::FromSqlError::OutOfRange(value))
            }
        })
    }
}

impl ChatId {
    /// Get ephemeral message timer value in seconds.
    pub async fn get_ephemeral_timer(self, context: &Context) -> Result<Timer> {
        let timer = context
            .sql
            .query_get_value(
                "SELECT ephemeral_timer FROM chats WHERE id=?;",
                paramsv![self],
            )
            .await?;
        Ok(timer.unwrap_or_default())
    }

    /// Set ephemeral timer value without sending a message.
    ///
    /// Used when a message arrives indicating that someone else has
    /// changed the timer value for a chat.
    pub(crate) async fn inner_set_ephemeral_timer(
        self,
        context: &Context,
        timer: Timer,
    ) -> Result<()> {
        ensure!(!self.is_special(), "Invalid chat ID");

        context
            .sql
            .execute(
                "UPDATE chats
             SET ephemeral_timer=?
             WHERE id=?;",
                paramsv![timer, self],
            )
            .await?;

        context.emit_event(EventType::ChatEphemeralTimerModified {
            chat_id: self,
            timer,
        });
        Ok(())
    }

    /// Set ephemeral message timer value in seconds.
    ///
    /// If timer value is 0, disable ephemeral message timer.
    pub async fn set_ephemeral_timer(self, context: &Context, timer: Timer) -> Result<()> {
        if timer == self.get_ephemeral_timer(context).await? {
            return Ok(());
        }
        self.inner_set_ephemeral_timer(context, timer).await?;
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(stock_ephemeral_timer_changed(context, timer, ContactId::SELF).await);
        msg.param.set_cmd(SystemMessage::EphemeralTimerChanged);
        if let Err(err) = send_msg(context, self, &mut msg).await {
            error!(
                context,
                "Failed to send a message about ephemeral message timer change: {:?}", err
            );
        }
        Ok(())
    }
}

/// Returns a stock message saying that ephemeral timer is changed to `timer` by `from_id`.
pub(crate) async fn stock_ephemeral_timer_changed(
    context: &Context,
    timer: Timer,
    from_id: ContactId,
) -> String {
    match timer {
        Timer::Disabled => stock_str::msg_ephemeral_timer_disabled(context, from_id).await,
        Timer::Enabled { duration } => match duration {
            0..=59 => {
                stock_str::msg_ephemeral_timer_enabled(context, timer.to_string(), from_id).await
            }
            60 => stock_str::msg_ephemeral_timer_minute(context, from_id).await,
            61..=3599 => {
                stock_str::msg_ephemeral_timer_minutes(
                    context,
                    format!("{}", (f64::from(duration) / 6.0).round() / 10.0),
                    from_id,
                )
                .await
            }
            3600 => stock_str::msg_ephemeral_timer_hour(context, from_id).await,
            3601..=86399 => {
                stock_str::msg_ephemeral_timer_hours(
                    context,
                    format!("{}", (f64::from(duration) / 360.0).round() / 10.0),
                    from_id,
                )
                .await
            }
            86400 => stock_str::msg_ephemeral_timer_day(context, from_id).await,
            86401..=604_799 => {
                stock_str::msg_ephemeral_timer_days(
                    context,
                    format!("{}", (f64::from(duration) / 8640.0).round() / 10.0),
                    from_id,
                )
                .await
            }
            604_800 => stock_str::msg_ephemeral_timer_week(context, from_id).await,
            _ => {
                stock_str::msg_ephemeral_timer_weeks(
                    context,
                    format!("{}", (f64::from(duration) / 60480.0).round() / 10.0),
                    from_id,
                )
                .await
            }
        },
    }
}

impl MsgId {
    /// Returns ephemeral message timer value for the message.
    pub(crate) async fn ephemeral_timer(self, context: &Context) -> Result<Timer> {
        let res = match context
            .sql
            .query_get_value(
                "SELECT ephemeral_timer FROM msgs WHERE id=?",
                paramsv![self],
            )
            .await?
        {
            None | Some(0) => Timer::Disabled,
            Some(duration) => Timer::Enabled { duration },
        };
        Ok(res)
    }

    /// Starts ephemeral message timer for the message if it is not started yet.
    pub(crate) async fn start_ephemeral_timer(self, context: &Context) -> Result<()> {
        if let Timer::Enabled { duration } = self.ephemeral_timer(context).await? {
            let ephemeral_timestamp = time().saturating_add(duration.into());

            context
                .sql
                .execute(
                    "UPDATE msgs SET ephemeral_timestamp = ? \
                WHERE (ephemeral_timestamp == 0 OR ephemeral_timestamp > ?) \
                AND id = ?",
                    paramsv![ephemeral_timestamp, ephemeral_timestamp, self],
                )
                .await?;
            context.interrupt_ephemeral_task().await;
        }
        Ok(())
    }
}

pub(crate) async fn start_ephemeral_timers_msgids(
    context: &Context,
    msg_ids: &[MsgId],
) -> Result<()> {
    let now = time();
    let count = context
        .sql
        .execute(
            format!(
                "UPDATE msgs SET ephemeral_timestamp = ? + ephemeral_timer
         WHERE (ephemeral_timestamp == 0 OR ephemeral_timestamp > ? + ephemeral_timer) AND ephemeral_timer > 0
         AND id IN ({})",
                sql::repeat_vars(msg_ids.len())
            ),
            rusqlite::params_from_iter(
                std::iter::once(&now as &dyn crate::ToSql)
                    .chain(std::iter::once(&now as &dyn crate::ToSql))
                    .chain(params_iter(msg_ids)),
            ),
        )
        .await?;
    if count > 0 {
        context.interrupt_ephemeral_task().await;
    }
    Ok(())
}

/// Deletes messages which are expired according to
/// `delete_device_after` setting or `ephemeral_timestamp` column.
///
/// Returns true if any message is deleted, so caller can emit
/// MsgsChanged event. If nothing has been deleted, returns
/// false. This function does not emit the MsgsChanged event itself,
/// because it is also called when chatlist is reloaded, and emitting
/// MsgsChanged there will cause infinite reload loop.
pub(crate) async fn delete_expired_messages(context: &Context, now: i64) -> Result<()> {
    let mut updated = context
        .sql
        .execute(
            // If you change which information is removed here, also change MsgId::trash() and
            // which information dc_receive_imf::add_parts() still adds to the db if the chat_id is TRASH
            r#"
UPDATE msgs
SET 
  chat_id=?, txt='', subject='', txt_raw='', 
  mime_headers='', from_id=0, to_id=0, param=''
WHERE
  ephemeral_timestamp != 0
  AND ephemeral_timestamp <= ?
  AND chat_id != ?
"#,
            paramsv![DC_CHAT_ID_TRASH, now, DC_CHAT_ID_TRASH],
        )
        .await
        .context("update failed")?
        > 0;

    if let Some(delete_device_after) = context.get_config_delete_device_after().await? {
        let self_chat_id = ChatId::lookup_by_contact(context, ContactId::SELF)
            .await?
            .unwrap_or_default();
        let device_chat_id = ChatId::lookup_by_contact(context, ContactId::DEVICE)
            .await?
            .unwrap_or_default();

        let threshold_timestamp = now.saturating_sub(delete_device_after);

        // Delete expired messages
        //
        // Only update the rows that have to be updated, to avoid emitting
        // unnecessary "chat modified" events.
        let rows_modified = context
            .sql
            .execute(
                "UPDATE msgs \
             SET chat_id = ?, txt = '', subject='', txt_raw='', \
                 mime_headers='', from_id=0, to_id=0, param='' \
             WHERE timestamp < ? \
             AND chat_id > ? \
             AND chat_id != ? \
             AND chat_id != ?",
                paramsv![
                    DC_CHAT_ID_TRASH,
                    threshold_timestamp,
                    DC_CHAT_ID_LAST_SPECIAL,
                    self_chat_id,
                    device_chat_id
                ],
            )
            .await
            .context("deleted update failed")?;

        updated |= rows_modified > 0;
    }

    if updated {
        context.emit_msgs_changed_without_ids();
    }

    Ok(())
}

/// Calculates the next timestamp when a message will be deleted due to
/// `delete_device_after` setting being set.
async fn next_delete_device_after_timestamp(context: &Context) -> Result<Option<i64>> {
    if let Some(delete_device_after) = context.get_config_delete_device_after().await? {
        let self_chat_id = ChatId::lookup_by_contact(context, ContactId::SELF)
            .await?
            .unwrap_or_default();
        let device_chat_id = ChatId::lookup_by_contact(context, ContactId::DEVICE)
            .await?
            .unwrap_or_default();

        let oldest_message_timestamp: Option<i64> = context
            .sql
            .query_get_value(
                r#"
                SELECT min(timestamp)
                FROM msgs
                WHERE chat_id > ?
                  AND chat_id != ?
                  AND chat_id != ?;
                "#,
                paramsv![DC_CHAT_ID_TRASH, self_chat_id, device_chat_id],
            )
            .await?;

        Ok(oldest_message_timestamp.map(|x| x.saturating_add(delete_device_after)))
    } else {
        Ok(None)
    }
}

/// Calculates next timestamp when expiration of some message will happen.
///
/// Expiration can happen either because user has set `delete_device_after` setting or because the
/// message itself has an ephemeral timer.
async fn next_expiration_timestamp(context: &Context) -> Option<i64> {
    let ephemeral_timestamp: Option<i64> = match context
        .sql
        .query_get_value(
            r#"
            SELECT min(ephemeral_timestamp)
            FROM msgs
            WHERE ephemeral_timestamp != 0
              AND chat_id != ?;
            "#,
            paramsv![DC_CHAT_ID_TRASH], // Trash contains already deleted messages, skip them
        )
        .await
    {
        Err(err) => {
            warn!(context, "Can't calculate next ephemeral timeout: {}", err);
            None
        }
        Ok(ephemeral_timestamp) => ephemeral_timestamp,
    };

    let delete_device_after_timestamp: Option<i64> =
        match next_delete_device_after_timestamp(context).await {
            Err(err) => {
                warn!(
                    context,
                    "Can't calculate timestamp of the next message expiration: {}", err
                );
                None
            }
            Ok(timestamp) => timestamp,
        };

    ephemeral_timestamp
        .into_iter()
        .chain(delete_device_after_timestamp.into_iter())
        .min()
}

pub(crate) async fn ephemeral_loop(context: &Context, interrupt_receiver: Receiver<()>) {
    loop {
        let ephemeral_timestamp = next_expiration_timestamp(context).await;

        let now = SystemTime::now();
        let until = if let Some(ephemeral_timestamp) = ephemeral_timestamp {
            UNIX_EPOCH
                + Duration::from_secs(ephemeral_timestamp.try_into().unwrap_or(u64::MAX))
                + Duration::from_secs(1)
        } else {
            // no messages to be deleted for now, wait long for one to occur
            now + Duration::from_secs(86400)
        };

        if let Ok(duration) = until.duration_since(now) {
            info!(
                context,
                "Ephemeral loop waiting for deletion in {} or interrupt",
                duration_to_str(duration)
            );
            if timeout(duration, interrupt_receiver.recv()).await.is_ok() {
                // received an interruption signal, recompute waiting time (if any)
                continue;
            }
        }

        delete_expired_messages(context, time())
            .await
            .ok_or_log(context);
    }
}

/// Schedules expired IMAP messages for deletion.
pub(crate) async fn delete_expired_imap_messages(context: &Context) -> Result<()> {
    let now = time();

    let (threshold_timestamp, threshold_timestamp_extended) =
        match context.get_config_delete_server_after().await? {
            None => (0, 0),
            Some(delete_server_after) => (
                now - delete_server_after,
                now - max(delete_server_after, MIN_DELETE_SERVER_AFTER),
            ),
        };

    context
        .sql
        .execute(
            "UPDATE imap
             SET target=''
             WHERE rfc724_mid IN (
               SELECT rfc724_mid FROM msgs
               WHERE ((download_state = 0 AND timestamp < ?) OR
                      (download_state != 0 AND timestamp < ?) OR
                      (ephemeral_timestamp != 0 AND ephemeral_timestamp <= ?))
             )",
            paramsv![threshold_timestamp, threshold_timestamp_extended, now],
        )
        .await?;

    Ok(())
}

/// Start ephemeral timers for seen messages if they are not started
/// yet.
///
/// It is possible that timers are not started due to a missing or
/// failed `MsgId.start_ephemeral_timer()` call, either in the current
/// or previous version of Delta Chat.
///
/// This function is supposed to be called in the background,
/// e.g. from housekeeping task.
pub(crate) async fn start_ephemeral_timers(context: &Context) -> Result<()> {
    context
        .sql
        .execute(
            "UPDATE msgs \
    SET ephemeral_timestamp = ? + ephemeral_timer \
    WHERE ephemeral_timer > 0 \
    AND ephemeral_timestamp = 0 \
    AND state NOT IN (?, ?, ?)",
            paramsv![
                time(),
                MessageState::InFresh,
                MessageState::InNoticed,
                MessageState::OutDraft
            ],
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dc_receive_imf::dc_receive_imf;
    use crate::dc_tools::MAX_SECONDS_TO_LEND_FROM_FUTURE;
    use crate::download::DownloadState;
    use crate::test_utils::TestContext;
    use crate::{
        chat::{self, Chat, ChatItem},
        dc_tools::IsNoneOrEmpty,
    };

    #[async_std::test]
    async fn test_stock_ephemeral_messages() {
        let context = TestContext::new().await;

        assert_eq!(
            stock_ephemeral_timer_changed(&context, Timer::Disabled, ContactId::SELF).await,
            "Message deletion timer is disabled by me."
        );

        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 1 },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 1 s by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 30 },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 30 s by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 60 },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 1 minute by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 90 },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 1.5 minutes by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 30 * 60 },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 30 minutes by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 60 * 60 },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 1 hour by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 5400 },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 1.5 hours by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled {
                    duration: 2 * 60 * 60
                },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 2 hours by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled {
                    duration: 24 * 60 * 60
                },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 1 day by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled {
                    duration: 2 * 24 * 60 * 60
                },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 2 days by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled {
                    duration: 7 * 24 * 60 * 60
                },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 1 week by me."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled {
                    duration: 4 * 7 * 24 * 60 * 60
                },
                ContactId::SELF
            )
            .await,
            "Message deletion timer is set to 4 weeks by me."
        );
    }

    /// Test enabling and disabling ephemeral timer remotely.
    #[async_std::test]
    async fn test_ephemeral_enable_disable() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let chat_alice = alice.create_chat(&bob).await.id;
        let chat_bob = bob.create_chat(&alice).await.id;

        chat_alice
            .set_ephemeral_timer(&alice.ctx, Timer::Enabled { duration: 60 })
            .await?;
        let sent = alice.pop_sent_msg().await;
        bob.recv_msg(&sent).await;
        assert_eq!(
            chat_bob.get_ephemeral_timer(&bob.ctx).await?,
            Timer::Enabled { duration: 60 }
        );

        chat_alice
            .set_ephemeral_timer(&alice.ctx, Timer::Disabled)
            .await?;
        let sent = alice.pop_sent_msg().await;
        bob.recv_msg(&sent).await;
        assert_eq!(
            chat_bob.get_ephemeral_timer(&bob.ctx).await?,
            Timer::Disabled
        );

        Ok(())
    }

    /// Test that timer is enabled even if the message explicitly enabling the timer is lost.
    #[async_std::test]
    async fn test_ephemeral_enable_lost() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let chat_alice = alice.create_chat(&bob).await.id;
        let chat_bob = bob.create_chat(&alice).await.id;

        // Alice enables the timer.
        chat_alice
            .set_ephemeral_timer(&alice.ctx, Timer::Enabled { duration: 60 })
            .await?;
        assert_eq!(
            chat_alice.get_ephemeral_timer(&alice.ctx).await?,
            Timer::Enabled { duration: 60 }
        );
        // The message enabling the timer is lost.
        let _sent = alice.pop_sent_msg().await;
        assert_eq!(
            chat_bob.get_ephemeral_timer(&bob.ctx).await?,
            Timer::Disabled,
        );

        // Alice sends a text message.
        let mut msg = Message::new(Viewtype::Text);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        // Bob receives text message and enables the timer, even though explicit timer update was
        // lost previously.
        bob.recv_msg(&sent).await;
        assert_eq!(
            chat_bob.get_ephemeral_timer(&bob.ctx).await?,
            Timer::Enabled { duration: 60 }
        );

        Ok(())
    }

    /// Test that Alice replying to the chat without a timer at the same time as Bob enables the
    /// timer does not result in disabling the timer on the Bob's side.
    #[async_std::test]
    async fn test_ephemeral_timer_rollback() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let chat_alice = alice.create_chat(&bob).await.id;
        let chat_bob = bob.create_chat(&alice).await.id;

        // Alice sends message to Bob
        let mut msg = Message::new(Viewtype::Text);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;
        bob.recv_msg(&sent).await;

        // Alice sends second message to Bob, with no timer
        let mut msg = Message::new(Viewtype::Text);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        assert_eq!(
            chat_bob.get_ephemeral_timer(&bob.ctx).await?,
            Timer::Disabled
        );

        // Bob sets ephemeral timer and sends a message about timer change
        chat_bob
            .set_ephemeral_timer(&bob.ctx, Timer::Enabled { duration: 60 })
            .await?;
        let sent_timer_change = bob.pop_sent_msg().await;

        assert_eq!(
            chat_bob.get_ephemeral_timer(&bob.ctx).await?,
            Timer::Enabled { duration: 60 }
        );

        // Bob receives message from Alice.
        // Alice message has no timer. However, Bob should not disable timer,
        // because Alice replies to old message.
        bob.recv_msg(&sent).await;

        assert_eq!(
            chat_alice.get_ephemeral_timer(&alice.ctx).await?,
            Timer::Disabled
        );
        assert_eq!(
            chat_bob.get_ephemeral_timer(&bob.ctx).await?,
            Timer::Enabled { duration: 60 }
        );

        // Alice receives message from Bob
        alice.recv_msg(&sent_timer_change).await;

        assert_eq!(
            chat_alice.get_ephemeral_timer(&alice.ctx).await?,
            Timer::Enabled { duration: 60 }
        );

        // Bob disables the chat timer.
        // Note that the last message in the Bob's chat is from Alice and has no timer,
        // but the chat timer is enabled.
        chat_bob
            .set_ephemeral_timer(&bob.ctx, Timer::Disabled)
            .await?;
        alice.recv_msg(&bob.pop_sent_msg().await).await;
        assert_eq!(
            chat_alice.get_ephemeral_timer(&alice.ctx).await?,
            Timer::Disabled
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_ephemeral_delete_msgs() -> Result<()> {
        let t = TestContext::new_alice().await;
        let self_chat = t.get_self_chat().await;

        assert_eq!(next_expiration_timestamp(&t).await, None);

        t.send_text(self_chat.id, "Saved message, which we delete manually")
            .await;
        let msg = t.get_last_msg_in(self_chat.id).await;
        msg.id.delete_from_db(&t).await?;
        check_msg_is_deleted(&t, &self_chat, msg.id).await;

        self_chat
            .id
            .set_ephemeral_timer(&t, Timer::Enabled { duration: 3600 })
            .await
            .unwrap();

        // Send a saved message which will be deleted after 3600s
        let now = time();
        let msg = t.send_text(self_chat.id, "Message text").await;

        check_msg_will_be_deleted(&t, msg.sender_msg_id, &self_chat, now + 3599, time() + 3601)
            .await
            .unwrap();

        // Set DeleteDeviceAfter to 1800s. Thend send a saved message which will
        // still be deleted after 3600s because DeleteDeviceAfter doesn't apply to saved messages.
        t.set_config(Config::DeleteDeviceAfter, Some("1800"))
            .await?;

        let now = time();
        let msg = t.send_text(self_chat.id, "Message text").await;

        check_msg_will_be_deleted(&t, msg.sender_msg_id, &self_chat, now + 3559, time() + 3601)
            .await
            .unwrap();

        // Send a message to Bob which will be deleted after 1800s because of DeleteDeviceAfter.
        let bob_chat = t.create_chat_with_contact("", "bob@example.net").await;
        let now = time();
        let msg = t.send_text(bob_chat.id, "Message text").await;

        check_msg_will_be_deleted(
            &t,
            msg.sender_msg_id,
            &bob_chat,
            now + 1799,
            // The message may appear to be sent MAX_SECONDS_TO_LEND_FROM_FUTURE later and
            // therefore be deleted MAX_SECONDS_TO_LEND_FROM_FUTURE later.
            time() + 1801 + MAX_SECONDS_TO_LEND_FROM_FUTURE,
        )
        .await
        .unwrap();

        // Enable ephemeral messages with Bob -> message will be deleted after 60s.
        // This tests that the message is deleted at min(ephemeral deletion time, DeleteDeviceAfter deletion time).
        bob_chat
            .id
            .set_ephemeral_timer(&t, Timer::Enabled { duration: 60 })
            .await?;

        let now = time();
        let msg = t.send_text(bob_chat.id, "Message text").await;

        check_msg_will_be_deleted(&t, msg.sender_msg_id, &bob_chat, now + 59, time() + 61)
            .await
            .unwrap();

        Ok(())
    }

    async fn check_msg_will_be_deleted(
        t: &TestContext,
        msg_id: MsgId,
        chat: &Chat,
        not_deleted_at: i64,
        deleted_at: i64,
    ) -> Result<()> {
        let next_expiration = next_expiration_timestamp(t).await.unwrap();

        assert!(next_expiration > not_deleted_at);
        delete_expired_messages(t, not_deleted_at).await?;

        let loaded = Message::load_from_db(t, msg_id).await?;
        assert_eq!(loaded.text.unwrap(), "Message text");
        assert_eq!(loaded.chat_id, chat.id);

        assert!(next_expiration < deleted_at);
        delete_expired_messages(t, deleted_at).await?;

        let loaded = Message::load_from_db(t, msg_id).await?;
        assert_eq!(loaded.text.unwrap(), "");
        assert_eq!(loaded.chat_id, DC_CHAT_ID_TRASH);

        // Check that the msg was deleted locally.
        check_msg_is_deleted(t, chat, msg_id).await;

        Ok(())
    }

    async fn check_msg_is_deleted(t: &TestContext, chat: &Chat, msg_id: MsgId) {
        let chat_items = chat::get_chat_msgs(t, chat.id, 0).await.unwrap();
        // Check that the chat is empty except for possibly info messages:
        for item in &chat_items {
            if let ChatItem::Message { msg_id } = item {
                let msg = Message::load_from_db(t, *msg_id).await.unwrap();
                assert!(msg.is_info())
            }
        }

        // Check that if there is a message left, the text and metadata are gone
        if let Ok(msg) = Message::load_from_db(t, msg_id).await {
            assert_eq!(msg.from_id, ContactId::UNDEFINED);
            assert_eq!(msg.to_id, ContactId::UNDEFINED);
            assert!(msg.text.is_none_or_empty(), "{:?}", msg.text);
            let rawtxt: Option<String> = t
                .sql
                .query_get_value("SELECT txt_raw FROM msgs WHERE id=?;", paramsv![msg_id])
                .await
                .unwrap();
            assert!(rawtxt.is_none_or_empty(), "{:?}", rawtxt);
        }
    }

    #[async_std::test]
    async fn test_delete_expired_imap_messages() -> Result<()> {
        let t = TestContext::new_alice().await;
        const HOUR: i64 = 60 * 60;
        let now = time();
        for (id, timestamp, ephemeral_timestamp) in &[
            (900, now - 2 * HOUR, 0),
            (1000, now - 23 * HOUR - MIN_DELETE_SERVER_AFTER, 0),
            (1010, now - 23 * HOUR, 0),
            (1020, now - 21 * HOUR, 0),
            (1030, now - 19 * HOUR, 0),
            (2000, now - 18 * HOUR, now - HOUR),
            (2020, now - 17 * HOUR, now + HOUR),
        ] {
            let message_id = id.to_string();
            t.sql
                   .execute(
                       "INSERT INTO msgs (id, rfc724_mid, timestamp, ephemeral_timestamp) VALUES (?,?,?,?);",
                       paramsv![id, message_id, timestamp, ephemeral_timestamp],
                   )
                   .await?;
            t.sql
                   .execute(
                       "INSERT INTO imap (rfc724_mid, folder, uid, target) VALUES (?,'INBOX',?, 'INBOX');",
                       paramsv![message_id, id],
                   )
                   .await?;
        }

        async fn test_marked_for_deletion(context: &Context, id: u32) -> Result<()> {
            assert_eq!(
                context
                    .sql
                    .count(
                        "SELECT COUNT(*) FROM imap WHERE target='' AND rfc724_mid=?",
                        paramsv![id.to_string()],
                    )
                    .await?,
                1
            );
            Ok(())
        }

        async fn remove_uid(context: &Context, id: u32) -> Result<()> {
            context
                .sql
                .execute(
                    "DELETE FROM imap WHERE rfc724_mid=?",
                    paramsv![id.to_string()],
                )
                .await?;
            Ok(())
        }

        // This should mark message 2000 for deletion.
        delete_expired_imap_messages(&t).await?;
        test_marked_for_deletion(&t, 2000).await?;
        remove_uid(&t, 2000).await?;
        // No other messages are marked for deletion.
        assert_eq!(
            t.sql
                .count("SELECT COUNT(*) FROM imap WHERE target=''", paramsv![],)
                .await?,
            0
        );

        t.set_config(Config::DeleteServerAfter, Some(&*(25 * HOUR).to_string()))
            .await?;
        delete_expired_imap_messages(&t).await?;
        test_marked_for_deletion(&t, 1000).await?;

        MsgId::new(1000)
            .update_download_state(&t, DownloadState::Available)
            .await?;
        t.sql
            .execute(
                "UPDATE imap SET target=folder WHERE rfc724_mid='1000'",
                paramsv![],
            )
            .await?;
        delete_expired_imap_messages(&t).await?;
        test_marked_for_deletion(&t, 1000).await?; // Delete downloadable anyway.
        remove_uid(&t, 1000).await?;

        t.set_config(Config::DeleteServerAfter, Some(&*(22 * HOUR).to_string()))
            .await?;
        delete_expired_imap_messages(&t).await?;
        test_marked_for_deletion(&t, 1010).await?;
        t.sql
            .execute(
                "UPDATE imap SET target=folder WHERE rfc724_mid='1010'",
                paramsv![],
            )
            .await?;

        MsgId::new(1010)
            .update_download_state(&t, DownloadState::Available)
            .await?;
        delete_expired_imap_messages(&t).await?;
        // Keep downloadable for now.
        assert_eq!(
            t.sql
                .count("SELECT COUNT(*) FROM imap WHERE target=''", paramsv![],)
                .await?,
            0
        );

        Ok(())
    }

    // Regression test for a bug in the timer rollback protection.
    #[async_std::test]
    async fn test_ephemeral_timer_references() -> Result<()> {
        let alice = TestContext::new_alice().await;

        // Message with Message-ID <first@example.com> and no timer is received.
        dc_receive_imf(
            &alice,
            b"From: Bob <bob@example.com>\n\
                    To: Alice <alice@example.org>\n\
                    Chat-Version: 1.0\n\
                    Subject: Subject\n\
                    Message-ID: <first@example.com>\n\
                    Date: Sun, 22 Mar 2020 00:10:00 +0000\n\
                    \n\
                    hello\n",
            false,
        )
        .await?;

        let msg = alice.get_last_msg().await;
        let chat_id = msg.chat_id;
        assert_eq!(chat_id.get_ephemeral_timer(&alice).await?, Timer::Disabled);

        // Message with Message-ID <second@example.com> is received.
        dc_receive_imf(
            &alice,
            b"From: Bob <bob@example.com>\n\
                    To: Alice <alice@example.org>\n\
                    Chat-Version: 1.0\n\
                    Subject: Subject\n\
                    Message-ID: <second@example.com>\n\
                    Date: Sun, 22 Mar 2020 00:11:00 +0000\n\
                    Ephemeral-Timer: 60\n\
                    \n\
                    second message\n",
            false,
        )
        .await?;
        assert_eq!(
            chat_id.get_ephemeral_timer(&alice).await?,
            Timer::Enabled { duration: 60 }
        );
        let msg = alice.get_last_msg().await;

        // Message is deleted from the database when its timer expires.
        msg.id.delete_from_db(&alice).await?;

        // Message with Message-ID <third@example.com>, referencing <first@example.com> and
        // <second@example.com>, is received.  The message <second@example.come> is not in the
        // database anymore, so the timer should be applied unconditionally without rollback
        // protection.
        //
        // Previously Delta Chat fallen back to using <first@example.com> in this case and
        // compared received timer value to the timer value of the <first@examle.com>. Because
        // their timer values are the same ("disabled"), Delta Chat assumed that the timer was not
        // changed explicitly and the change should be ignored.
        //
        // The message also contains a quote of the first message to test that only References:
        // header and not In-Reply-To: is consulted by the rollback protection.
        dc_receive_imf(
            &alice,
            b"From: Bob <bob@example.com>\n\
                    To: Alice <alice@example.org>\n\
                    Chat-Version: 1.0\n\
                    Subject: Subject\n\
                    Message-ID: <third@example.com>\n\
                    Date: Sun, 22 Mar 2020 00:12:00 +0000\n\
                    References: <first@example.com> <second@example.com>\n\
                    In-Reply-To: <first@example.com>\n\
                    \n\
                    > hello\n",
            false,
        )
        .await?;

        let msg = alice.get_last_msg().await;
        assert_eq!(
            msg.chat_id.get_ephemeral_timer(&alice).await?,
            Timer::Disabled
        );

        Ok(())
    }
}
