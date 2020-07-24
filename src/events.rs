//! # Events specification

use async_std::path::PathBuf;
use async_std::sync::{channel, Receiver, Sender, TrySendError};
use strum::EnumProperty;

use crate::chat::ChatId;
use crate::ephemeral::Timer as EphemeralTimer;
use crate::message::MsgId;

#[derive(Debug)]
pub struct Events {
    receiver: Receiver<Event>,
    sender: Sender<Event>,
}

impl Default for Events {
    fn default() -> Self {
        let (sender, receiver) = channel(1_000);

        Self { receiver, sender }
    }
}

impl Events {
    pub fn emit(&self, event: Event) {
        match self.sender.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(event)) => {
                // when we are full, we pop remove the oldest event and push on the new one
                let _ = self.receiver.try_recv();

                // try again
                self.emit(event);
            }
            Err(TrySendError::Disconnected(_)) => {
                unreachable!("unable to emit event, channel disconnected");
            }
        }
    }

    /// Retrieve the event emitter.
    pub fn get_emitter(&self) -> EventEmitter {
        EventEmitter(self.receiver.clone())
    }
}

#[derive(Debug, Clone)]
pub struct EventEmitter(Receiver<Event>);

impl EventEmitter {
    /// Blocking recv of an event. Return `None` if the `Sender` has been droped.
    pub fn recv_sync(&self) -> Option<Event> {
        async_std::task::block_on(self.recv())
    }

    /// Blocking async recv of an event. Return `None` if the `Sender` has been droped.
    pub async fn recv(&self) -> Option<Event> {
        // TODO: change once we can use async channels internally.
        self.0.recv().await.ok()
    }
}

impl Event {
    /// Returns the corresponding Event id.
    pub fn as_id(&self) -> i32 {
        self.get_str("id")
            .expect("missing id")
            .parse()
            .expect("invalid id")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumProperty)]
pub enum Event {
    /// The library-user may write an informational string to the log.
    /// Passed to the callback given to dc_context_new().
    /// This event should not be reported to the end-user using a popup or something like that.
    #[strum(props(id = "100"))]
    Info(String),

    /// Emitted when SMTP connection is established and login was successful.
    #[strum(props(id = "101"))]
    SmtpConnected(String),

    /// Emitted when IMAP connection is established and login was successful.
    #[strum(props(id = "102"))]
    ImapConnected(String),

    /// Emitted when a message was successfully sent to the SMTP server.
    #[strum(props(id = "103"))]
    SmtpMessageSent(String),

    /// Emitted when an IMAP message has been marked as deleted
    #[strum(props(id = "104"))]
    ImapMessageDeleted(String),

    /// Emitted when an IMAP message has been moved
    #[strum(props(id = "105"))]
    ImapMessageMoved(String),

    /// Emitted when an IMAP folder was emptied
    #[strum(props(id = "106"))]
    ImapFolderEmptied(String),

    /// Emitted when an new file in the $BLOBDIR was created
    #[strum(props(id = "150"))]
    NewBlobFile(String),

    /// Emitted when an file in the $BLOBDIR was deleted
    #[strum(props(id = "151"))]
    DeletedBlobFile(String),

    /// The library-user should write a warning string to the log.
    /// Passed to the callback given to dc_context_new().
    ///
    /// This event should not be reported to the end-user using a popup or something like that.
    #[strum(props(id = "300"))]
    Warning(String),

    /// The library-user should report an error to the end-user.
    /// Passed to the callback given to dc_context_new().
    ///
    /// As most things are asynchronous, things may go wrong at any time and the user
    /// should not be disturbed by a dialog or so.  Instead, use a bubble or so.
    ///
    /// However, for ongoing processes (eg. configure())
    /// or for functions that are expected to fail (eg. dc_continue_key_transfer())
    /// it might be better to delay showing these events until the function has really
    /// failed (returned false). It should be sufficient to report only the *last* error
    /// in a messasge box then.
    #[strum(props(id = "400"))]
    Error(String),

    /// An action cannot be performed because there is no network available.
    ///
    /// The library will typically try over after a some time
    /// and when dc_maybe_network() is called.
    ///
    /// Network errors should be reported to users in a non-disturbing way,
    /// however, as network errors may come in a sequence,
    /// it is not useful to raise each an every error to the user.
    ///
    /// Moreover, if the UI detects that the device is offline,
    /// it is probably more useful to report this to the user
    /// instead of the string from data2.
    #[strum(props(id = "401"))]
    ErrorNetwork(String),

    /// An action cannot be performed because the user is not in the group.
    /// Reported eg. after a call to
    /// dc_set_chat_name(), dc_set_chat_profile_image(),
    /// dc_add_contact_to_chat(), dc_remove_contact_from_chat(),
    /// dc_send_text_msg() or another sending function.
    #[strum(props(id = "410"))]
    ErrorSelfNotInGroup(String),

    /// Messages or chats changed.  One or more messages or chats changed for various
    /// reasons in the database:
    /// - Messages sent, received or removed
    /// - Chats created, deleted or archived
    /// - A draft has been set
    #[strum(props(id = "2000"))]
    MsgsChanged { chat_id: ChatId, msg_id: MsgId },

    /// There is a fresh message. Typically, the user will show an notification
    /// when receiving this message.
    ///
    /// There is no extra #DC_EVENT_MSGS_CHANGED event send together with this event.
    #[strum(props(id = "2005"))]
    IncomingMsg { chat_id: ChatId, msg_id: MsgId },

    /// A single message is sent successfully. State changed from  DC_STATE_OUT_PENDING to
    /// DC_STATE_OUT_DELIVERED, see dc_msg_get_state().
    #[strum(props(id = "2010"))]
    MsgDelivered { chat_id: ChatId, msg_id: MsgId },

    /// A single message could not be sent. State changed from DC_STATE_OUT_PENDING or DC_STATE_OUT_DELIVERED to
    /// DC_STATE_OUT_FAILED, see dc_msg_get_state().
    #[strum(props(id = "2012"))]
    MsgFailed { chat_id: ChatId, msg_id: MsgId },

    /// A single message is read by the receiver. State changed from DC_STATE_OUT_DELIVERED to
    /// DC_STATE_OUT_MDN_RCVD, see dc_msg_get_state().
    #[strum(props(id = "2015"))]
    MsgRead { chat_id: ChatId, msg_id: MsgId },

    /// Chat changed.  The name or the image of a chat group was changed or members were added or removed.
    /// Or the verify state of a chat has changed.
    /// See dc_set_chat_name(), dc_set_chat_profile_image(), dc_add_contact_to_chat()
    /// and dc_remove_contact_from_chat().
    ///
    /// This event does not include ephemeral timer modification, which
    /// is a separate event.
    #[strum(props(id = "2020"))]
    ChatModified(ChatId),

    /// Chat ephemeral timer changed.
    #[strum(props(id = "2021"))]
    ChatEphemeralTimerModified {
        chat_id: ChatId,
        timer: EphemeralTimer,
    },

    /// Contact(s) created, renamed, blocked or deleted.
    ///
    /// @param data1 (int) If set, this is the contact_id of an added contact that should be selected.
    #[strum(props(id = "2030"))]
    ContactsChanged(Option<u32>),

    /// Location of one or more contact has changed.
    ///
    /// @param data1 (u32) contact_id of the contact for which the location has changed.
    ///     If the locations of several contacts have been changed,
    ///     eg. after calling dc_delete_all_locations(), this parameter is set to `None`.
    #[strum(props(id = "2035"))]
    LocationChanged(Option<u32>),

    /// Inform about the configuration progress started by configure().
    ///
    /// @param data1 (usize) 0=error, 1-999=progress in permille, 1000=success and done
    #[strum(props(id = "2041"))]
    ConfigureProgress(usize),

    /// Inform about the import/export progress started by imex().
    ///
    /// @param data1 (usize) 0=error, 1-999=progress in permille, 1000=success and done
    /// @param data2 0
    #[strum(props(id = "2051"))]
    ImexProgress(usize),

    /// A file has been exported. A file has been written by imex().
    /// This event may be sent multiple times by a single call to imex().
    ///
    /// A typical purpose for a handler of this event may be to make the file public to some system
    /// services.
    ///
    /// @param data2 0
    #[strum(props(id = "2052"))]
    ImexFileWritten(PathBuf),

    /// Progress information of a secure-join handshake from the view of the inviter
    /// (Alice, the person who shows the QR code).
    ///
    /// These events are typically sent after a joiner has scanned the QR code
    /// generated by dc_get_securejoin_qr().
    ///
    /// @param data1 (int) ID of the contact that wants to join.
    /// @param data2 (int) Progress as:
    ///     300=vg-/vc-request received, typically shown as "bob@addr joins".
    ///     600=vg-/vc-request-with-auth received, vg-member-added/vc-contact-confirm sent, typically shown as "bob@addr verified".
    ///     800=vg-member-added-received received, shown as "bob@addr securely joined GROUP", only sent for the verified-group-protocol.
    ///     1000=Protocol finished for this contact.
    #[strum(props(id = "2060"))]
    SecurejoinInviterProgress { contact_id: u32, progress: usize },

    /// Progress information of a secure-join handshake from the view of the joiner
    /// (Bob, the person who scans the QR code).
    /// The events are typically sent while dc_join_securejoin(), which
    /// may take some time, is executed.
    /// @param data1 (int) ID of the inviting contact.
    /// @param data2 (int) Progress as:
    ///     400=vg-/vc-request-with-auth sent, typically shown as "alice@addr verified, introducing myself."
    ///     (Bob has verified alice and waits until Alice does the same for him)
    #[strum(props(id = "2061"))]
    SecurejoinJoinerProgress { contact_id: u32, progress: usize },
}
