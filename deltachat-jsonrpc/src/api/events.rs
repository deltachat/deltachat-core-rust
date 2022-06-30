use deltachat::{Event, EventType};
use serde::Serialize;
use serde_json::{json, Value};
use typescript_type_def::TypeDef;

pub fn event_to_json_rpc_notification(event: Event) -> Value {
    let (field1, field2): (Value, Value) = match &event.typ {
        // events with a single string in field1
        EventType::Info(txt)
        | EventType::SmtpConnected(txt)
        | EventType::ImapConnected(txt)
        | EventType::SmtpMessageSent(txt)
        | EventType::ImapMessageDeleted(txt)
        | EventType::ImapMessageMoved(txt)
        | EventType::NewBlobFile(txt)
        | EventType::DeletedBlobFile(txt)
        | EventType::Warning(txt)
        | EventType::Error(txt)
        | EventType::ErrorSelfNotInGroup(txt) => (json!(txt), Value::Null),
        EventType::ImexFileWritten(path) => (json!(path.to_str()), Value::Null),
        // single number
        EventType::MsgsNoticed(chat_id) | EventType::ChatModified(chat_id) => {
            (json!(chat_id), Value::Null)
        }
        EventType::ImexProgress(progress) => (json!(progress), Value::Null),
        // both fields contain numbers
        EventType::MsgsChanged { chat_id, msg_id }
        | EventType::IncomingMsg { chat_id, msg_id }
        | EventType::MsgDelivered { chat_id, msg_id }
        | EventType::MsgFailed { chat_id, msg_id }
        | EventType::MsgRead { chat_id, msg_id } => (json!(chat_id), json!(msg_id)),
        EventType::ChatEphemeralTimerModified { chat_id, timer } => (json!(chat_id), json!(timer)),
        EventType::SecurejoinInviterProgress {
            contact_id,
            progress,
        }
        | EventType::SecurejoinJoinerProgress {
            contact_id,
            progress,
        } => (json!(contact_id), json!(progress)),
        // field 1 number or null
        EventType::ContactsChanged(maybe_number) | EventType::LocationChanged(maybe_number) => (
            match maybe_number {
                Some(number) => json!(number),
                None => Value::Null,
            },
            Value::Null,
        ),
        // number and maybe string
        EventType::ConfigureProgress { progress, comment } => (
            json!(progress),
            match comment {
                Some(content) => json!(content),
                None => Value::Null,
            },
        ),
        EventType::ConnectivityChanged => (Value::Null, Value::Null),
        EventType::SelfavatarChanged => (Value::Null, Value::Null),
        EventType::WebxdcStatusUpdate {
            msg_id,
            status_update_serial,
        } => (json!(msg_id), json!(status_update_serial)),
    };

    let id: EventTypeName = event.typ.into();
    json!({
        "id": id,
        "contextId": event.id,
        "field1": field1,
        "field2": field2
    })
}

#[derive(Serialize, TypeDef)]
pub enum EventTypeName {
    Info,
    SmtpConnected,
    ImapConnected,
    SmtpMessageSent,
    ImapMessageDeleted,
    ImapMessageMoved,
    NewBlobFile,
    DeletedBlobFile,
    Warning,
    Error,
    ErrorSelfNotInGroup,
    MsgsChanged,
    IncomingMsg,
    MsgsNoticed,
    MsgDelivered,
    MsgFailed,
    MsgRead,
    ChatModified,
    ChatEphemeralTimerModified,
    ContactsChanged,
    LocationChanged,
    ConfigureProgress,
    ImexProgress,
    ImexFileWritten,
    SecurejoinInviterProgress,
    SecurejoinJoinerProgress,
    ConnectivityChanged,
    SelfavatarChanged,
    WebxdcStatusUpdate,
}

impl From<EventType> for EventTypeName {
    fn from(event: EventType) -> Self {
        use EventTypeName::*;
        match event {
            EventType::Info(_) => Info,
            EventType::SmtpConnected(_) => SmtpConnected,
            EventType::ImapConnected(_) => ImapConnected,
            EventType::SmtpMessageSent(_) => SmtpMessageSent,
            EventType::ImapMessageDeleted(_) => ImapMessageDeleted,
            EventType::ImapMessageMoved(_) => ImapMessageMoved,
            EventType::NewBlobFile(_) => NewBlobFile,
            EventType::DeletedBlobFile(_) => DeletedBlobFile,
            EventType::Warning(_) => Warning,
            EventType::Error(_) => Error,
            EventType::ErrorSelfNotInGroup(_) => ErrorSelfNotInGroup,
            EventType::MsgsChanged { .. } => MsgsChanged,
            EventType::IncomingMsg { .. } => IncomingMsg,
            EventType::MsgsNoticed(_) => MsgsNoticed,
            EventType::MsgDelivered { .. } => MsgDelivered,
            EventType::MsgFailed { .. } => MsgFailed,
            EventType::MsgRead { .. } => MsgRead,
            EventType::ChatModified(_) => ChatModified,
            EventType::ChatEphemeralTimerModified { .. } => ChatEphemeralTimerModified,
            EventType::ContactsChanged(_) => ContactsChanged,
            EventType::LocationChanged(_) => LocationChanged,
            EventType::ConfigureProgress { .. } => ConfigureProgress,
            EventType::ImexProgress(_) => ImexProgress,
            EventType::ImexFileWritten(_) => ImexFileWritten,
            EventType::SecurejoinInviterProgress { .. } => SecurejoinInviterProgress,
            EventType::SecurejoinJoinerProgress { .. } => SecurejoinJoinerProgress,
            EventType::ConnectivityChanged => ConnectivityChanged,
            EventType::SelfavatarChanged => SelfavatarChanged,
            EventType::WebxdcStatusUpdate { .. } => WebxdcStatusUpdate,
        }
    }
}

#[cfg(test)]
#[test]
fn generate_events_ts_types_definition() {
    let events = {
        let mut buf = Vec::new();
        let options = typescript_type_def::DefinitionFileOptions {
            root_namespace: None,
            ..typescript_type_def::DefinitionFileOptions::default()
        };
        typescript_type_def::write_definition_file::<_, EventTypeName>(&mut buf, options).unwrap();
        String::from_utf8(buf).unwrap()
    };
    std::fs::write("typescript/generated/events.ts", events).unwrap();
}
