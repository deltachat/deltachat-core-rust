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
//! When Delta Chat deletes the message locally, it moves the message
//! to the trash chat and removes actual message contents. Messages in
//! the trash chat are called "tombstones" and track the Message-ID to
//! prevent accidental redownloading of the message from the server,
//! e.g. in case of UID validity change.
//!
//! Vice versa, when Delta Chat deletes the message from the server,
//! it removes IMAP folder and UID row from the `imap` table, but
//! keeps the message in the `msgs` table.
//!
//! Delta Chat eventually removes tombstones from the `msgs` table,
//! leaving no trace of the message, when it thinks there are no more
//! copies of the message stored on the server, i.e. when there is no
//! corresponding `imap` table entry. This is done in the
//! `prune_tombstones()` procedure during housekeeping.
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

use std::cmp::max;
use std::collections::BTreeSet;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{ensure, Context as _, Result};
use async_channel::Receiver;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::chat::{send_msg, ChatId, ChatIdBlocked};
use crate::constants::{DC_CHAT_ID_LAST_SPECIAL, DC_CHAT_ID_TRASH};
use crate::contact::ContactId;
use crate::context::Context;
use crate::download::MIN_DELETE_SERVER_AFTER;
use crate::events::EventType;
use crate::location;
use crate::log::LogExt;
use crate::message::{Message, MessageState, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::stock_str;
use crate::tools::{duration_to_str, time, SystemTime};

/// Ephemeral timer value.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum Timer {
    /// Timer is disabled.
    Disabled,

    /// Timer is enabled.
    Enabled {
        /// Timer duration in seconds.
        ///
        /// The value cannot be 0.
        duration: u32,
    },
}

impl Timer {
    /// Converts epehmeral timer value to integer.
    ///
    /// If the timer is disabled, return 0.
    pub fn to_u32(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::Enabled { duration } => duration,
        }
    }

    /// Converts integer to ephemeral timer value.
    ///
    /// 0 value is treated as disabled timer.
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

impl fmt::Display for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_u32())
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
                "SELECT IFNULL(ephemeral_timer, 0) FROM chats WHERE id=?",
                (self,),
            )
            .await?
            .with_context(|| format!("Chat {self} not found"))?;
        Ok(timer)
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
                (timer, self),
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

        if self.is_promoted(context).await? {
            let mut msg = Message::new_text(
                stock_ephemeral_timer_changed(context, timer, ContactId::SELF).await,
            );
            msg.param.set_cmd(SystemMessage::EphemeralTimerChanged);
            if let Err(err) = send_msg(context, self, &mut msg).await {
                error!(
                    context,
                    "Failed to send a message about ephemeral message timer change: {:?}", err
                );
            }
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
                stock_str::msg_ephemeral_timer_enabled(context, &timer.to_string(), from_id).await
            }
            60 => stock_str::msg_ephemeral_timer_minute(context, from_id).await,
            61..=3599 => {
                stock_str::msg_ephemeral_timer_minutes(
                    context,
                    &format!("{}", (f64::from(duration) / 6.0).round() / 10.0),
                    from_id,
                )
                .await
            }
            3600 => stock_str::msg_ephemeral_timer_hour(context, from_id).await,
            3601..=86399 => {
                stock_str::msg_ephemeral_timer_hours(
                    context,
                    &format!("{}", (f64::from(duration) / 360.0).round() / 10.0),
                    from_id,
                )
                .await
            }
            86400 => stock_str::msg_ephemeral_timer_day(context, from_id).await,
            86401..=604_799 => {
                stock_str::msg_ephemeral_timer_days(
                    context,
                    &format!("{}", (f64::from(duration) / 8640.0).round() / 10.0),
                    from_id,
                )
                .await
            }
            604_800 => stock_str::msg_ephemeral_timer_week(context, from_id).await,
            _ => {
                stock_str::msg_ephemeral_timer_weeks(
                    context,
                    &format!("{}", (f64::from(duration) / 60480.0).round() / 10.0),
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
            .query_get_value("SELECT ephemeral_timer FROM msgs WHERE id=?", (self,))
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
                    (ephemeral_timestamp, ephemeral_timestamp, self),
                )
                .await?;
            context.scheduler.interrupt_ephemeral_task().await;
        }
        Ok(())
    }
}

pub(crate) async fn start_ephemeral_timers_msgids(
    context: &Context,
    msg_ids: &[MsgId],
) -> Result<()> {
    let now = time();
    let should_interrupt =
    context
        .sql
        .transaction(move |transaction| {
            let mut should_interrupt = false;
            let mut stmt =
                transaction.prepare(
                    "UPDATE msgs SET ephemeral_timestamp = ?1 + ephemeral_timer
                     WHERE (ephemeral_timestamp == 0 OR ephemeral_timestamp > ?1 + ephemeral_timer) AND ephemeral_timer > 0
                     AND id=?2")?;
            for msg_id in msg_ids {
                should_interrupt |= stmt.execute((now, msg_id))? > 0;
            }
            Ok(should_interrupt)
        }).await?;
    if should_interrupt {
        context.scheduler.interrupt_ephemeral_task().await;
    }
    Ok(())
}

/// Starts ephemeral timer for all messages in the chat.
///
/// This should be called when chat is marked as noticed.
pub(crate) async fn start_chat_ephemeral_timers(context: &Context, chat_id: ChatId) -> Result<()> {
    let now = time();
    let should_interrupt = context
        .sql
        .execute(
            "UPDATE msgs SET ephemeral_timestamp = ?1 + ephemeral_timer
             WHERE chat_id = ?2
             AND ephemeral_timer > 0
             AND (ephemeral_timestamp == 0 OR ephemeral_timestamp > ?1 + ephemeral_timer)",
            (now, chat_id),
        )
        .await?
        > 0;
    if should_interrupt {
        context.scheduler.interrupt_ephemeral_task().await;
    }
    Ok(())
}

/// Selects messages which are expired according to
/// `delete_device_after` setting or `ephemeral_timestamp` column.
///
/// For each message a row ID, chat id, viewtype and location ID is returned.
async fn select_expired_messages(
    context: &Context,
    now: i64,
) -> Result<Vec<(MsgId, ChatId, Viewtype, u32)>> {
    let mut rows = context
        .sql
        .query_map(
            r#"
SELECT id, chat_id, type, location_id
FROM msgs
WHERE
  ephemeral_timestamp != 0
  AND ephemeral_timestamp <= ?
  AND chat_id != ?
"#,
            (now, DC_CHAT_ID_TRASH),
            |row| {
                let id: MsgId = row.get("id")?;
                let chat_id: ChatId = row.get("chat_id")?;
                let viewtype: Viewtype = row.get("type")?;
                let location_id: u32 = row.get("location_id")?;
                Ok((id, chat_id, viewtype, location_id))
            },
            |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
        )
        .await?;

    if let Some(delete_device_after) = context.get_config_delete_device_after().await? {
        let self_chat_id = ChatIdBlocked::lookup_by_contact(context, ContactId::SELF)
            .await?
            .map(|c| c.id)
            .unwrap_or_default();
        let device_chat_id = ChatIdBlocked::lookup_by_contact(context, ContactId::DEVICE)
            .await?
            .map(|c| c.id)
            .unwrap_or_default();

        let threshold_timestamp = now.saturating_sub(delete_device_after);

        let rows_expired = context
            .sql
            .query_map(
                r#"
SELECT id, chat_id, type, location_id
FROM msgs
WHERE
  timestamp < ?1
  AND timestamp_rcvd < ?1
  AND chat_id > ?
  AND chat_id != ?
  AND chat_id != ?
"#,
                (
                    threshold_timestamp,
                    DC_CHAT_ID_LAST_SPECIAL,
                    self_chat_id,
                    device_chat_id,
                ),
                |row| {
                    let id: MsgId = row.get("id")?;
                    let chat_id: ChatId = row.get("chat_id")?;
                    let viewtype: Viewtype = row.get("type")?;
                    let location_id: u32 = row.get("location_id")?;
                    Ok((id, chat_id, viewtype, location_id))
                },
                |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .await?;

        rows.extend(rows_expired);
    }

    Ok(rows)
}

/// Deletes messages which are expired according to
/// `delete_device_after` setting or `ephemeral_timestamp` column.
///
/// Emits relevant `MsgsChanged` and `WebxdcInstanceDeleted` events
/// if messages are deleted.
pub(crate) async fn delete_expired_messages(context: &Context, now: i64) -> Result<()> {
    let rows = select_expired_messages(context, now).await?;

    if !rows.is_empty() {
        info!(context, "Attempting to delete {} messages.", rows.len());

        let (msgs_changed, webxdc_deleted) = context
            .sql
            .transaction(|transaction| {
                let mut msgs_changed = Vec::with_capacity(rows.len());
                let mut webxdc_deleted = Vec::new();

                // If you change which information is removed here, also change MsgId::trash() and
                // which information receive_imf::add_parts() still adds to the db if the chat_id is TRASH
                for (msg_id, chat_id, viewtype, location_id) in rows {
                    transaction.execute(
                        "UPDATE msgs
                     SET chat_id=?, txt='', txt_normalized=NULL, subject='', txt_raw='',
                         mime_headers='', from_id=0, to_id=0, param=''
                     WHERE id=?",
                        (DC_CHAT_ID_TRASH, msg_id),
                    )?;

                    if location_id > 0 {
                        transaction.execute(
                            "DELETE FROM locations WHERE independent=1 AND id=?",
                            (location_id,),
                        )?;
                    }

                    msgs_changed.push((chat_id, msg_id));
                    if viewtype == Viewtype::Webxdc {
                        webxdc_deleted.push(msg_id)
                    }
                }
                Ok((msgs_changed, webxdc_deleted))
            })
            .await?;

        let mut modified_chat_ids = BTreeSet::new();

        for (chat_id, msg_id) in msgs_changed {
            context.emit_event(EventType::MsgDeleted { chat_id, msg_id });
            modified_chat_ids.insert(chat_id);
        }

        for modified_chat_id in modified_chat_ids {
            context.emit_msgs_changed(modified_chat_id, MsgId::new(0));
        }

        for msg_id in webxdc_deleted {
            context.emit_event(EventType::WebxdcInstanceDeleted { msg_id });
        }
    }

    Ok(())
}

/// Calculates the next timestamp when a message will be deleted due to
/// `delete_device_after` setting being set.
async fn next_delete_device_after_timestamp(context: &Context) -> Result<Option<i64>> {
    if let Some(delete_device_after) = context.get_config_delete_device_after().await? {
        let self_chat_id = ChatIdBlocked::lookup_by_contact(context, ContactId::SELF)
            .await?
            .map(|c| c.id)
            .unwrap_or_default();
        let device_chat_id = ChatIdBlocked::lookup_by_contact(context, ContactId::DEVICE)
            .await?
            .map(|c| c.id)
            .unwrap_or_default();

        let oldest_message_timestamp: Option<i64> = context
            .sql
            .query_get_value(
                r#"
                SELECT min(max(timestamp, timestamp_rcvd))
                FROM msgs
                WHERE chat_id > ?
                  AND chat_id != ?
                  AND chat_id != ?
                HAVING count(*) > 0
                "#,
                (DC_CHAT_ID_TRASH, self_chat_id, device_chat_id),
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
              AND chat_id != ?
            HAVING count(*) > 0
            "#,
            (DC_CHAT_ID_TRASH,), // Trash contains already deleted messages, skip them
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
        .chain(delete_device_after_timestamp)
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
            match timeout(duration, interrupt_receiver.recv()).await {
                Ok(Ok(())) => {
                    // received an interruption signal, recompute waiting time (if any)
                    continue;
                }
                Ok(Err(err)) => {
                    warn!(
                        context,
                        "Interrupt channel closed, ephemeral loop exits now: {err:#}."
                    );
                    return;
                }
                Err(_err) => {
                    // Timeout.
                }
            }
        }

        delete_expired_messages(context, time())
            .await
            .log_err(context)
            .ok();

        location::delete_expired(context, time())
            .await
            .log_err(context)
            .ok();
    }
}

/// Schedules expired IMAP messages for deletion.
pub(crate) async fn delete_expired_imap_messages(context: &Context) -> Result<()> {
    let now = time();

    let (threshold_timestamp, threshold_timestamp_extended) =
        match context.get_config_delete_server_after().await? {
            None => (0, 0),
            Some(delete_server_after) => (
                match delete_server_after {
                    // Guarantee immediate deletion.
                    0 => i64::MAX,
                    _ => now - delete_server_after,
                },
                now - max(delete_server_after, MIN_DELETE_SERVER_AFTER),
            ),
        };
    let target = context.get_delete_msgs_target().await?;

    context
        .sql
        .execute(
            "UPDATE imap
             SET target=?
             WHERE rfc724_mid IN (
               SELECT rfc724_mid FROM msgs
               WHERE ((download_state = 0 AND timestamp < ?) OR
                      (download_state != 0 AND timestamp < ?) OR
                      (ephemeral_timestamp != 0 AND ephemeral_timestamp <= ?))
             )",
            (
                &target,
                threshold_timestamp,
                threshold_timestamp_extended,
                now,
            ),
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
            (
                time(),
                MessageState::InFresh,
                MessageState::InNoticed,
                MessageState::OutDraft,
            ),
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{marknoticed_chat, set_muted, ChatVisibility, MuteDuration};
    use crate::config::Config;
    use crate::constants::DC_CHAT_ID_ARCHIVED_LINK;
    use crate::download::DownloadState;
    use crate::location;
    use crate::message::markseen_msgs;
    use crate::receive_imf::receive_imf;
    use crate::test_utils::{TestContext, TestContextManager};
    use crate::timesmearing::MAX_SECONDS_TO_LEND_FROM_FUTURE;
    use crate::{
        chat::{self, create_group_chat, send_text_msg, Chat, ChatItem, ProtectionStatus},
        tools::IsNoneOrEmpty,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_stock_ephemeral_messages() {
        let context = TestContext::new().await;

        assert_eq!(
            stock_ephemeral_timer_changed(&context, Timer::Disabled, ContactId::SELF).await,
            "You disabled message deletion timer."
        );

        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 1 },
                ContactId::SELF
            )
            .await,
            "You set message deletion timer to 1 s."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 30 },
                ContactId::SELF
            )
            .await,
            "You set message deletion timer to 30 s."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 60 },
                ContactId::SELF
            )
            .await,
            "You set message deletion timer to 1 minute."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 90 },
                ContactId::SELF
            )
            .await,
            "You set message deletion timer to 1.5 minutes."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 30 * 60 },
                ContactId::SELF
            )
            .await,
            "You set message deletion timer to 30 minutes."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 60 * 60 },
                ContactId::SELF
            )
            .await,
            "You set message deletion timer to 1 hour."
        );
        assert_eq!(
            stock_ephemeral_timer_changed(
                &context,
                Timer::Enabled { duration: 5400 },
                ContactId::SELF
            )
            .await,
            "You set message deletion timer to 1.5 hours."
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
            "You set message deletion timer to 2 hours."
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
            "You set message deletion timer to 1 day."
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
            "You set message deletion timer to 2 days."
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
            "You set message deletion timer to 1 week."
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
            "You set message deletion timer to 4 weeks."
        );
    }

    /// Test enabling and disabling ephemeral timer remotely.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

    /// Test that enabling ephemeral timer in unpromoted group does not send a message.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ephemeral_unpromoted() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let chat_id =
            create_group_chat(&alice, ProtectionStatus::Unprotected, "Group name").await?;

        // Group is unpromoted, the timer can be changed without sending a message.
        assert!(chat_id.is_unpromoted(&alice).await?);
        chat_id
            .set_ephemeral_timer(&alice, Timer::Enabled { duration: 60 })
            .await?;
        let sent = alice.pop_sent_msg_opt(Duration::from_secs(1)).await;
        assert!(sent.is_none());
        assert_eq!(
            chat_id.get_ephemeral_timer(&alice).await?,
            Timer::Enabled { duration: 60 }
        );

        // Promote the group.
        send_text_msg(&alice, chat_id, "hi!".to_string()).await?;
        assert!(chat_id.is_promoted(&alice).await?);
        let sent = alice.pop_sent_msg_opt(Duration::from_secs(1)).await;
        assert!(sent.is_some());

        chat_id
            .set_ephemeral_timer(&alice.ctx, Timer::Disabled)
            .await?;
        let sent = alice.pop_sent_msg_opt(Duration::from_secs(1)).await;
        assert!(sent.is_some());
        assert_eq!(chat_id.get_ephemeral_timer(&alice).await?, Timer::Disabled);

        Ok(())
    }

    /// Test that timer is enabled even if the message explicitly enabling the timer is lost.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ephemeral_timer_rollback() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let chat_alice = alice.create_chat(&bob).await.id;
        let chat_bob = bob.create_chat(&alice).await.id;

        // Alice sends message to Bob
        let mut msg = Message::new(Viewtype::Text);
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;
        bob.recv_msg(&sent).await;

        // Alice sends second message to Bob, with no timer
        let mut msg = Message::new(Viewtype::Text);
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ephemeral_delete_msgs() -> Result<()> {
        let t = TestContext::new_alice().await;
        let self_chat = t.get_self_chat().await;

        assert_eq!(next_expiration_timestamp(&t).await, None);

        t.send_text(self_chat.id, "Saved message, which we delete manually")
            .await;
        let msg = t.get_last_msg_in(self_chat.id).await;
        msg.id.trash(&t, false).await?;
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

        // Set DeleteDeviceAfter to 1800s. Then send a saved message which will
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
        assert!(!loaded.text.is_empty());
        assert_eq!(loaded.chat_id, chat.id);

        assert!(next_expiration < deleted_at);
        delete_expired_messages(t, deleted_at).await?;
        t.evtracker
            .get_matching(|evt| {
                if let EventType::MsgDeleted {
                    msg_id: event_msg_id,
                    ..
                } = evt
                {
                    *event_msg_id == msg_id
                } else {
                    false
                }
            })
            .await;

        let loaded = Message::load_from_db_optional(t, msg_id).await?;
        assert!(loaded.is_none());

        // Check that the msg was deleted locally.
        check_msg_is_deleted(t, chat, msg_id).await;

        Ok(())
    }

    async fn check_msg_is_deleted(t: &TestContext, chat: &Chat, msg_id: MsgId) {
        let chat_items = chat::get_chat_msgs(t, chat.id).await.unwrap();
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
            assert_eq!(msg.text, "");
            let rawtxt: Option<String> = t
                .sql
                .query_get_value("SELECT txt_raw FROM msgs WHERE id=?;", (msg_id,))
                .await
                .unwrap();
            assert!(rawtxt.is_none_or_empty(), "{rawtxt:?}");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
            (3000, now + HOUR, 0),
        ] {
            let message_id = id.to_string();
            t.sql
                   .execute(
                       "INSERT INTO msgs (id, rfc724_mid, timestamp, ephemeral_timestamp) VALUES (?,?,?,?);",
                       (id, &message_id, timestamp, ephemeral_timestamp),
                   )
                   .await?;
            t.sql
                   .execute(
                       "INSERT INTO imap (rfc724_mid, folder, uid, target) VALUES (?,'INBOX',?, 'INBOX');",
                       (&message_id, id),
                   )
                   .await?;
        }

        async fn test_marked_for_deletion(context: &Context, id: u32) -> Result<()> {
            assert_eq!(
                context
                    .sql
                    .count(
                        "SELECT COUNT(*) FROM imap WHERE target='' AND rfc724_mid=?",
                        (id.to_string(),),
                    )
                    .await?,
                1
            );
            Ok(())
        }

        async fn remove_uid(context: &Context, id: u32) -> Result<()> {
            context
                .sql
                .execute("DELETE FROM imap WHERE rfc724_mid=?", (id.to_string(),))
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
                .count("SELECT COUNT(*) FROM imap WHERE target=''", ())
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
            .execute("UPDATE imap SET target=folder WHERE rfc724_mid='1000'", ())
            .await?;
        delete_expired_imap_messages(&t).await?;
        test_marked_for_deletion(&t, 1000).await?; // Delete downloadable anyway.
        remove_uid(&t, 1000).await?;

        t.set_config(Config::DeleteServerAfter, Some(&*(22 * HOUR).to_string()))
            .await?;
        delete_expired_imap_messages(&t).await?;
        test_marked_for_deletion(&t, 1010).await?;
        t.sql
            .execute("UPDATE imap SET target=folder WHERE rfc724_mid='1010'", ())
            .await?;

        MsgId::new(1010)
            .update_download_state(&t, DownloadState::Available)
            .await?;
        delete_expired_imap_messages(&t).await?;
        // Keep downloadable for now.
        assert_eq!(
            t.sql
                .count("SELECT COUNT(*) FROM imap WHERE target=''", ())
                .await?,
            0
        );

        t.set_config(Config::DeleteServerAfter, Some("1")).await?;
        delete_expired_imap_messages(&t).await?;
        test_marked_for_deletion(&t, 3000).await?;

        Ok(())
    }

    // Regression test for a bug in the timer rollback protection.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ephemeral_timer_references() -> Result<()> {
        let alice = TestContext::new_alice().await;

        // Message with Message-ID <first@example.com> and no timer is received.
        receive_imf(
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
        receive_imf(
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

        // Message is deleted when its timer expires.
        msg.id.trash(&alice, false).await?;

        // Message with Message-ID <third@example.com>, referencing <first@example.com> and
        // <second@example.com>, is received.  The message <second@example.come> is not in the
        // database anymore, so the timer should be applied unconditionally without rollback
        // protection.
        //
        // Previously Delta Chat fallen back to using <first@example.com> in this case and
        // compared received timer value to the timer value of the <first@example.com>. Because
        // their timer values are the same ("disabled"), Delta Chat assumed that the timer was not
        // changed explicitly and the change should be ignored.
        //
        // The message also contains a quote of the first message to test that only References:
        // header and not In-Reply-To: is consulted by the rollback protection.
        receive_imf(
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

    // Tests that if we are offline for a time longer than the ephemeral timer duration, the message
    // is deleted from the chat but is still in the "smtp" table, i.e. will be sent upon a
    // successful reconnection.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ephemeral_msg_offline() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let chat = alice
            .create_chat_with_contact("Bob", "bob@example.org")
            .await;
        let duration = 60;
        chat.id
            .set_ephemeral_timer(&alice, Timer::Enabled { duration })
            .await?;
        let mut msg = Message::new_text("hi".to_string());
        assert!(chat::send_msg_sync(&alice, chat.id, &mut msg)
            .await
            .is_err());
        let stmt = "SELECT COUNT(*) FROM smtp WHERE msg_id=?";
        assert!(alice.sql.exists(stmt, (msg.id,)).await?);
        let now = time();
        check_msg_will_be_deleted(&alice, msg.id, &chat, now, now + i64::from(duration) + 1)
            .await?;
        assert!(alice.sql.exists(stmt, (msg.id,)).await?);

        Ok(())
    }

    /// Tests that POI location is deleted when ephemeral message expires.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ephemeral_poi_location() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let bob = &tcm.bob().await;

        let chat = alice.create_chat(bob).await;

        let duration = 60;
        chat.id
            .set_ephemeral_timer(alice, Timer::Enabled { duration })
            .await?;
        let sent = alice.pop_sent_msg().await;
        bob.recv_msg(&sent).await;

        let mut poi_msg = Message::new_text("Here".to_string());
        poi_msg.set_location(10.0, 20.0);

        let alice_sent_message = alice.send_msg(chat.id, &mut poi_msg).await;
        let bob_received_message = bob.recv_msg(&alice_sent_message).await;
        markseen_msgs(bob, vec![bob_received_message.id]).await?;

        for account in [alice, bob] {
            let locations = location::get_range(account, None, None, 0, 0).await?;
            assert_eq!(locations.len(), 1);
        }

        SystemTime::shift(Duration::from_secs(100));

        for account in [alice, bob] {
            delete_expired_messages(account, time()).await?;
            let locations = location::get_range(account, None, None, 0, 0).await?;
            assert_eq!(locations.len(), 0);
        }

        Ok(())
    }

    /// Tests that `.get_ephemeral_timer()` returns an error for invalid chat ID.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ephemeral_timer_wrong_chat_id() -> Result<()> {
        let context = TestContext::new().await;
        let chat_id = ChatId::new(12345);
        assert!(chat_id.get_ephemeral_timer(&context).await.is_err());

        Ok(())
    }

    /// Tests that ephemeral timer is started when the chat is noticed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_noticed_ephemeral_timer() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let bob = &tcm.bob().await;

        let chat = alice.create_chat(bob).await;
        let duration = 60;
        chat.id
            .set_ephemeral_timer(alice, Timer::Enabled { duration })
            .await?;
        let bob_received_message = tcm.send_recv(alice, bob, "Hello!").await;

        marknoticed_chat(bob, bob_received_message.chat_id).await?;
        SystemTime::shift(Duration::from_secs(100));

        delete_expired_messages(bob, time()).await?;

        assert!(Message::load_from_db_optional(bob, bob_received_message.id)
            .await?
            .is_none());
        Ok(())
    }

    /// Tests that archiving the chat starts ephemeral timer.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_archived_ephemeral_timer() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let bob = &tcm.bob().await;

        let chat = alice.create_chat(bob).await;
        let duration = 60;
        chat.id
            .set_ephemeral_timer(alice, Timer::Enabled { duration })
            .await?;
        let bob_received_message = tcm.send_recv(alice, bob, "Hello!").await;

        bob_received_message
            .chat_id
            .set_visibility(bob, ChatVisibility::Archived)
            .await?;
        SystemTime::shift(Duration::from_secs(100));

        delete_expired_messages(bob, time()).await?;

        assert!(Message::load_from_db_optional(bob, bob_received_message.id)
            .await?
            .is_none());

        // Bob mutes the chat so it is not unarchived.
        set_muted(bob, bob_received_message.chat_id, MuteDuration::Forever).await?;

        // Now test that for already archived chat
        // timer is started if all archived chats are marked as noticed.
        let bob_received_message_2 = tcm.send_recv(alice, bob, "Hello again!").await;
        assert_eq!(bob_received_message_2.state, MessageState::InFresh);

        marknoticed_chat(bob, DC_CHAT_ID_ARCHIVED_LINK).await?;
        SystemTime::shift(Duration::from_secs(100));

        delete_expired_messages(bob, time()).await?;

        assert!(
            Message::load_from_db_optional(bob, bob_received_message_2.id)
                .await?
                .is_none()
        );

        Ok(())
    }
}
