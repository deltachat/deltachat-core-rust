use deltachat::{Event, EventType};
use serde::Serialize;
use serde_json::{json, Value};
use typescript_type_def::TypeDef;

pub fn event_to_json_rpc_notification(event: Event) -> Value {
    let id: JSONRPCEventType = event.typ.into();
    json!({
        "event": id,
        "contextId": event.id,
    })
}

#[derive(Serialize, TypeDef)]
#[serde(tag = "type", rename = "Event")]
pub enum JSONRPCEventType {
    /// The library-user may write an informational string to the log.
    ///
    /// This event should *not* be reported to the end-user using a popup or something like
    /// that.
    Info {
        msg: String,
    },

    /// Emitted when SMTP connection is established and login was successful.
    SmtpConnected {
        msg: String,
    },

    /// Emitted when IMAP connection is established and login was successful.
    ImapConnected {
        msg: String,
    },

    /// Emitted when a message was successfully sent to the SMTP server.
    SmtpMessageSent {
        msg: String,
    },

    /// Emitted when an IMAP message has been marked as deleted
    ImapMessageDeleted {
        msg: String,
    },

    /// Emitted when an IMAP message has been moved
    ImapMessageMoved {
        msg: String,
    },

    /// Emitted when an new file in the $BLOBDIR was created
    NewBlobFile {
        file: String,
    },

    /// Emitted when an file in the $BLOBDIR was deleted
    DeletedBlobFile {
        file: String,
    },

    /// The library-user should write a warning string to the log.
    ///
    /// This event should *not* be reported to the end-user using a popup or something like
    /// that.
    Warning {
        msg: String,
    },

    /// The library-user should report an error to the end-user.
    ///
    /// As most things are asynchronous, things may go wrong at any time and the user
    /// should not be disturbed by a dialog or so.  Instead, use a bubble or so.
    ///
    /// However, for ongoing processes (eg. configure())
    /// or for functions that are expected to fail (eg. autocryptContinueKeyTransfer())
    /// it might be better to delay showing these events until the function has really
    /// failed (returned false). It should be sufficient to report only the *last* error
    /// in a messasge box then.
    Error {
        msg: String,
    },

    /// An action cannot be performed because the user is not in the group.
    /// Reported eg. after a call to
    /// setChatName(), setChatProfileImage(),
    /// addContactToChat(), removeContactFromChat(),
    /// and messages sending functions.
    ErrorSelfNotInGroup {
        msg: String,
    },

    /// Messages or chats changed.  One or more messages or chats changed for various
    /// reasons in the database:
    /// - Messages sent, received or removed
    /// - Chats created, deleted or archived
    /// - A draft has been set
    ///
    /// `chatId` is set if only a single chat is affected by the changes, otherwise 0.
    /// `msgId` is set if only a single message is affected by the changes, otherwise 0.
    #[serde(rename_all = "camelCase")]
    MsgsChanged {
        chat_id: u32,
        msg_id: u32,
    },

    /// Reactions for the message changed.
    #[serde(rename_all = "camelCase")]
    ReactionsChanged {
        chat_id: u32,
        msg_id: u32,
        contact_id: u32,
    },

    /// There is a fresh message. Typically, the user will show an notification
    /// when receiving this message.
    ///
    /// There is no extra #DC_EVENT_MSGS_CHANGED event send together with this event.
    #[serde(rename_all = "camelCase")]
    IncomingMsg {
        chat_id: u32,
        msg_id: u32,
    },

    /// Downloading a bunch of messages just finished. This is an experimental
    /// event to allow the UI to only show one notification per message bunch,
    /// instead of cluttering the user with many notifications.
    ///
    /// msg_ids contains the message ids.
    #[serde(rename_all = "camelCase")]
    IncomingMsgBunch {
        msg_ids: Vec<u32>,
    },

    /// Messages were seen or noticed.
    /// chat id is always set.
    #[serde(rename_all = "camelCase")]
    MsgsNoticed {
        chat_id: u32,
    },

    /// A single message is sent successfully. State changed from  DC_STATE_OUT_PENDING to
    /// DC_STATE_OUT_DELIVERED, see `Message.state`.
    #[serde(rename_all = "camelCase")]
    MsgDelivered {
        chat_id: u32,
        msg_id: u32,
    },

    /// A single message could not be sent. State changed from DC_STATE_OUT_PENDING or DC_STATE_OUT_DELIVERED to
    /// DC_STATE_OUT_FAILED, see `Message.state`.
    #[serde(rename_all = "camelCase")]
    MsgFailed {
        chat_id: u32,
        msg_id: u32,
    },

    /// A single message is read by the receiver. State changed from DC_STATE_OUT_DELIVERED to
    /// DC_STATE_OUT_MDN_RCVD, see `Message.state`.
    #[serde(rename_all = "camelCase")]
    MsgRead {
        chat_id: u32,
        msg_id: u32,
    },

    /// Chat changed.  The name or the image of a chat group was changed or members were added or removed.
    /// Or the verify state of a chat has changed.
    /// See setChatName(), setChatProfileImage(), addContactToChat()
    /// and removeContactFromChat().
    ///
    /// This event does not include ephemeral timer modification, which
    /// is a separate event.
    #[serde(rename_all = "camelCase")]
    ChatModified {
        chat_id: u32,
    },

    /// Chat ephemeral timer changed.
    #[serde(rename_all = "camelCase")]
    ChatEphemeralTimerModified {
        chat_id: u32,
        timer: u32,
    },

    /// Contact(s) created, renamed, blocked or deleted.
    ///
    /// @param data1 (int) If set, this is the contact_id of an added contact that should be selected.
    #[serde(rename_all = "camelCase")]
    ContactsChanged {
        contact_id: Option<u32>,
    },

    /// Location of one or more contact has changed.
    ///
    /// @param data1 (u32) contact_id of the contact for which the location has changed.
    ///     If the locations of several contacts have been changed,
    ///     this parameter is set to `None`.
    #[serde(rename_all = "camelCase")]
    LocationChanged {
        contact_id: Option<u32>,
    },

    /// Inform about the configuration progress started by configure().
    ConfigureProgress {
        /// Progress.
        ///
        /// 0=error, 1-999=progress in permille, 1000=success and done
        progress: usize,

        /// Progress comment or error, something to display to the user.
        comment: Option<String>,
    },

    /// Inform about the import/export progress started by imex().
    ///
    /// @param data1 (usize) 0=error, 1-999=progress in permille, 1000=success and done
    /// @param data2 0
    #[serde(rename_all = "camelCase")]
    ImexProgress {
        progress: usize,
    },

    /// A file has been exported. A file has been written by imex().
    /// This event may be sent multiple times by a single call to imex().
    ///
    /// A typical purpose for a handler of this event may be to make the file public to some system
    /// services.
    ///
    /// @param data2 0
    #[serde(rename_all = "camelCase")]
    ImexFileWritten {
        path: String,
    },

    /// Progress information of a secure-join handshake from the view of the inviter
    /// (Alice, the person who shows the QR code).
    ///
    /// These events are typically sent after a joiner has scanned the QR code
    /// generated by getChatSecurejoinQrCodeSvg().
    ///
    /// @param data1 (int) ID of the contact that wants to join.
    /// @param data2 (int) Progress as:
    ///     300=vg-/vc-request received, typically shown as "bob@addr joins".
    ///     600=vg-/vc-request-with-auth received, vg-member-added/vc-contact-confirm sent, typically shown as "bob@addr verified".
    ///     800=vg-member-added-received received, shown as "bob@addr securely joined GROUP", only sent for the verified-group-protocol.
    ///     1000=Protocol finished for this contact.
    #[serde(rename_all = "camelCase")]
    SecurejoinInviterProgress {
        contact_id: u32,
        progress: usize,
    },

    /// Progress information of a secure-join handshake from the view of the joiner
    /// (Bob, the person who scans the QR code).
    /// The events are typically sent while secureJoin(), which
    /// may take some time, is executed.
    /// @param data1 (int) ID of the inviting contact.
    /// @param data2 (int) Progress as:
    ///     400=vg-/vc-request-with-auth sent, typically shown as "alice@addr verified, introducing myself."
    ///     (Bob has verified alice and waits until Alice does the same for him)
    #[serde(rename_all = "camelCase")]
    SecurejoinJoinerProgress {
        contact_id: u32,
        progress: usize,
    },

    /// The connectivity to the server changed.
    /// This means that you should refresh the connectivity view
    /// and possibly the connectivtiy HTML; see getConnectivity() and
    /// getConnectivityHtml() for details.
    ConnectivityChanged,

    SelfavatarChanged,

    #[serde(rename_all = "camelCase")]
    WebxdcStatusUpdate {
        msg_id: u32,
        status_update_serial: u32,
    },

    /// Inform that a message containing a webxdc instance has been deleted
    #[serde(rename_all = "camelCase")]
    WebxdcInstanceDeleted {
        msg_id: u32,
    },
    WebxdcBusyUpdating,
    WebxdcUpToDate,
    WebxdcUpdateStateChanged,
}

impl From<EventType> for JSONRPCEventType {
    fn from(event: EventType) -> Self {
        use JSONRPCEventType::*;
        match event {
            EventType::Info(msg) => Info { msg },
            EventType::SmtpConnected(msg) => SmtpConnected { msg },
            EventType::ImapConnected(msg) => ImapConnected { msg },
            EventType::SmtpMessageSent(msg) => SmtpMessageSent { msg },
            EventType::ImapMessageDeleted(msg) => ImapMessageDeleted { msg },
            EventType::ImapMessageMoved(msg) => ImapMessageMoved { msg },
            EventType::NewBlobFile(file) => NewBlobFile { file },
            EventType::DeletedBlobFile(file) => DeletedBlobFile { file },
            EventType::Warning(msg) => Warning { msg },
            EventType::Error(msg) => Error { msg },
            EventType::ErrorSelfNotInGroup(msg) => ErrorSelfNotInGroup { msg },
            EventType::MsgsChanged { chat_id, msg_id } => MsgsChanged {
                chat_id: chat_id.to_u32(),
                msg_id: msg_id.to_u32(),
            },
            EventType::ReactionsChanged {
                chat_id,
                msg_id,
                contact_id,
            } => ReactionsChanged {
                chat_id: chat_id.to_u32(),
                msg_id: msg_id.to_u32(),
                contact_id: contact_id.to_u32(),
            },
            EventType::IncomingMsg { chat_id, msg_id } => IncomingMsg {
                chat_id: chat_id.to_u32(),
                msg_id: msg_id.to_u32(),
            },
            EventType::IncomingMsgBunch { msg_ids } => IncomingMsgBunch {
                msg_ids: msg_ids.into_iter().map(|id| id.to_u32()).collect(),
            },
            EventType::MsgsNoticed(chat_id) => MsgsNoticed {
                chat_id: chat_id.to_u32(),
            },
            EventType::MsgDelivered { chat_id, msg_id } => MsgDelivered {
                chat_id: chat_id.to_u32(),
                msg_id: msg_id.to_u32(),
            },
            EventType::MsgFailed { chat_id, msg_id } => MsgFailed {
                chat_id: chat_id.to_u32(),
                msg_id: msg_id.to_u32(),
            },
            EventType::MsgRead { chat_id, msg_id } => MsgRead {
                chat_id: chat_id.to_u32(),
                msg_id: msg_id.to_u32(),
            },
            EventType::ChatModified(chat_id) => ChatModified {
                chat_id: chat_id.to_u32(),
            },
            EventType::ChatEphemeralTimerModified { chat_id, timer } => {
                ChatEphemeralTimerModified {
                    chat_id: chat_id.to_u32(),
                    timer: timer.to_u32(),
                }
            }
            EventType::ContactsChanged(contact) => ContactsChanged {
                contact_id: contact.map(|c| c.to_u32()),
            },
            EventType::LocationChanged(contact) => LocationChanged {
                contact_id: contact.map(|c| c.to_u32()),
            },
            EventType::ConfigureProgress { progress, comment } => {
                ConfigureProgress { progress, comment }
            }
            EventType::ImexProgress(progress) => ImexProgress { progress },
            EventType::ImexFileWritten(path) => ImexFileWritten {
                path: path.to_str().unwrap_or_default().to_owned(),
            },
            EventType::SecurejoinInviterProgress {
                contact_id,
                progress,
            } => SecurejoinInviterProgress {
                contact_id: contact_id.to_u32(),
                progress,
            },
            EventType::SecurejoinJoinerProgress {
                contact_id,
                progress,
            } => SecurejoinJoinerProgress {
                contact_id: contact_id.to_u32(),
                progress,
            },
            EventType::ConnectivityChanged => ConnectivityChanged,
            EventType::SelfavatarChanged => SelfavatarChanged,
            EventType::WebxdcStatusUpdate {
                msg_id,
                status_update_serial,
            } => WebxdcStatusUpdate {
                msg_id: msg_id.to_u32(),
                status_update_serial: status_update_serial.to_u32(),
            },
            EventType::WebxdcInstanceDeleted { msg_id } => WebxdcInstanceDeleted {
                msg_id: msg_id.to_u32(),
            },
            EventType::WebxdcUpdateStateChanged { .. } => WebxdcUpdateStateChanged,
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
        typescript_type_def::write_definition_file::<_, JSONRPCEventType>(&mut buf, options)
            .unwrap();
        String::from_utf8(buf).unwrap()
    };
    std::fs::write("typescript/generated/events.ts", events).unwrap();
}
