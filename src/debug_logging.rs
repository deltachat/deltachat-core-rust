use std::{path::PathBuf, sync::atomic};

use crate::{
    chat::ChatId,
    config::Config,
    context::Context,
    message::{Message, MsgId, Viewtype},
    param::Param,
    webxdc::StatusUpdateItem,
    Event, EventType,
};
use async_channel::Receiver;

/// Store all information needed to log an event to a webxdc.
pub struct DebugEventLogData {
    pub time: i64,
    pub msg_id: MsgId,
    pub event: EventType,
}

/// This loop should be send all log events by `Context::emit_event()` to forward them to the responsible
/// webxdc.
pub async fn debug_logging_loop(context: &Context, events: Receiver<DebugEventLogData>) {
    while let Ok(DebugEventLogData {
        time,
        msg_id,
        event,
    }) = events.recv().await
    {
        match context
            .write_status_update_inner(
                &msg_id,
                StatusUpdateItem {
                    payload: event.to_json(Some(time)),
                    info: None,
                    summary: None,
                    document: None,
                },
            )
            .await
        {
            Err(err) => {
                eprintln!("Can't log event to webxdc status update: {:#}", err);
            }
            Ok(serial) => {
                context.events.emit(Event {
                    id: context.id,
                    typ: EventType::WebxdcStatusUpdate {
                        msg_id,
                        status_update_serial: serial,
                    },
                });
            }
        }
    }
}

/// Set message as new logging webxdc if filename and chat_id fit
pub async fn maybe_set_logging_xdc(
    context: &Context,
    msg: &Message,
    chat_id: ChatId,
) -> anyhow::Result<()> {
    maybe_set_logging_xdc_inner(
        context,
        msg.get_viewtype(),
        chat_id,
        msg.param.get_path(Param::File, context),
        msg.get_id(),
    )
    .await?;

    Ok(())
}

/// Set message as new logging webxdc if filename and chat_id fit
pub async fn maybe_set_logging_xdc_inner(
    context: &Context,
    viewtype: Viewtype,
    chat_id: ChatId,
    file: anyhow::Result<Option<PathBuf>>,
    msg_id: MsgId,
) -> anyhow::Result<()> {
    if viewtype == Viewtype::Webxdc && chat_id.is_self_talk(context).await? {
        if let Ok(Some(file)) = file {
            if let Some(file_name) = file.file_name() {
                if file_name == "debug_logging.xdc" {
                    set_xdc_on_context(context, Some(msg_id.to_u32())).await;
                }
            }
        }
    }
    Ok(())
}

/// Set the webxdc contained in the msg as the current logging xdc on the context
/// Also save it to the database
pub async fn set_xdc_on_context(context: &Context, id: Option<u32>) {
    if context
        .sql
        .set_raw_config(
            Config::DebugLogging.as_ref(),
            id.map(|val| val.to_string()).as_deref(),
        )
        .await
        .is_ok()
    {
        context
            .debug_logging
            .store(id.unwrap_or_default(), atomic::Ordering::Relaxed);
        info!(context, "replacing logging webxdc");
    } else {
        warn!(
            context,
            "Couldn't set debug logging webxdc because of some sql error"
        )
    }
}
