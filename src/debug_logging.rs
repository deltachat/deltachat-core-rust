use crate::{
    chat::ChatId,
    config::Config,
    context::{Context, DebugLogging},
    message::{Message, MsgId, Viewtype},
    param::Param,
    webxdc::StatusUpdateItem,
    Event, EventType,
};
use async_channel::{self as channel, Receiver};
/// Forward log messages to logging webxdc
use std::path::PathBuf;
use tokio::task;

/// Store all information needed to log an event to a webxdc.
pub struct DebugEventLogData {
    pub time: i64,
    pub msg_id: MsgId,
    pub event: EventType,
}

/// Creates a loop which forwards all log messages send into the channel to the associated
/// logging xdc.
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
    if viewtype == Viewtype::Webxdc {
        if let Ok(Some(file)) = file {
            if let Some(file_name) = file.file_name() {
                if file_name == "debug_logging.xdc" && chat_id.is_self_talk(context).await? {
                    set_debug_logging_xdc(context, Some(msg_id)).await?;
                }
            }
        }
    }
    Ok(())
}

/// Set the webxdc contained in the msg as the current logging xdc on the context and save it to db
/// If id is a `None` value, disable debug logging
pub(crate) async fn set_debug_logging_xdc(ctx: &Context, id: Option<MsgId>) -> anyhow::Result<()> {
    match id {
        Some(msg_id) => {
            ctx.sql
                .set_raw_config(
                    Config::DebugLogging.as_ref(),
                    Some(msg_id.to_string().as_ref()),
                )
                .await?;
            let debug_logging = &mut *ctx.debug_logging.write().await;
            match debug_logging {
                // Switch logging xdc
                Some(debug_logging) => debug_logging.msg_id = msg_id,
                // Bootstrap background loop for message forwarding
                None => {
                    let (sender, debug_logging_recv) = channel::bounded(1000);
                    let loop_handle = {
                        let ctx = ctx.clone();
                        task::spawn(
                            async move { debug_logging_loop(&ctx, debug_logging_recv).await },
                        )
                    };
                    *debug_logging = Some(DebugLogging {
                        msg_id,
                        loop_handle,
                        sender,
                    });
                }
            }
            info!(ctx, "replacing logging webxdc");
        }
        // Delete current debug logging
        None => {
            ctx.sql
                .set_raw_config(Config::DebugLogging.as_ref(), None)
                .await?;
            *ctx.debug_logging.write().await = None;
        }
    }
    Ok(())
}
