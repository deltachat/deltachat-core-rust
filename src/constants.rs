//! Constants
#![allow(non_camel_case_types, dead_code)]

use lazy_static::lazy_static;

use deltachat_derive::*;

lazy_static! {
    pub static ref DC_VERSION_STR: String = env!("CARGO_PKG_VERSION").to_string();
}

#[repr(u8)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql)]
pub enum MoveState {
    Undefined = 0,
    Pending = 1,
    Stay = 2,
    Moving = 3,
}

// some defaults
const DC_E2EE_DEFAULT_ENABLED: i32 = 1;
pub const DC_MDNS_DEFAULT_ENABLED: i32 = 1;
const DC_INBOX_WATCH_DEFAULT: i32 = 1;
const DC_SENTBOX_WATCH_DEFAULT: i32 = 1;
const DC_MVBOX_WATCH_DEFAULT: i32 = 1;
const DC_MVBOX_MOVE_DEFAULT: i32 = 1;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(u8)]
pub enum Blocked {
    Not = 0,
    Manually = 1,
    Deaddrop = 2,
}

impl Default for Blocked {
    fn default() -> Self {
        Blocked::Not
    }
}

pub const DC_IMAP_SEEN: u32 = 0x1;

const DC_HANDSHAKE_CONTINUE_NORMAL_PROCESSING: i32 = 0x01;
pub const DC_HANDSHAKE_STOP_NORMAL_PROCESSING: i32 = 0x02;
pub const DC_HANDSHAKE_ADD_DELETE_JOB: i32 = 0x04;

pub const DC_GCL_ARCHIVED_ONLY: usize = 0x01;
pub const DC_GCL_NO_SPECIALS: usize = 0x02;
pub const DC_GCL_ADD_ALLDONE_HINT: usize = 0x04;

const DC_GCM_ADDDAYMARKER: usize = 0x01;

const DC_GCL_VERIFIED_ONLY: usize = 0x01;
pub const DC_GCL_ADD_SELF: usize = 0x02;

/// param1 is a directory where the keys are written to
const DC_IMEX_EXPORT_SELF_KEYS: usize = 1;
/// param1 is a directory where the keys are searched in and read from
const DC_IMEX_IMPORT_SELF_KEYS: usize = 2;
/// param1 is a directory where the backup is written to
const DC_IMEX_EXPORT_BACKUP: usize = 11;
/// param1 is the file with the backup to import
const DC_IMEX_IMPORT_BACKUP: usize = 12;

/// virtual chat showing all messages belonging to chats flagged with chats.blocked=2
pub(crate) const DC_CHAT_ID_DEADDROP: u32 = 1;
/// messages that should be deleted get this chat_id; the messages are deleted from the working thread later then. This is also needed as rfc724_mid should be preset as long as the message is not deleted on the server (otherwise it is downloaded again)
pub const DC_CHAT_ID_TRASH: u32 = 3;
/// a message is just in creation but not yet assigned to a chat (eg. we may need the message ID to set up blobs; this avoids unready message to be sent and shown)
const DC_CHAT_ID_MSGS_IN_CREATION: u32 = 4;
/// virtual chat showing all messages flagged with msgs.starred=2
const DC_CHAT_ID_STARRED: u32 = 5;
/// only an indicator in a chatlist
pub const DC_CHAT_ID_ARCHIVED_LINK: u32 = 6;
/// only an indicator in a chatlist
pub const DC_CHAT_ID_ALLDONE_HINT: u32 = 7;
/// larger chat IDs are "real" chats, their messages are "real" messages.
pub const DC_CHAT_ID_LAST_SPECIAL: u32 = 9;

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
    IntoStaticStr,
)]
#[repr(u32)]
pub enum Chattype {
    Undefined = 0,
    Single = 100,
    Group = 120,
    VerifiedGroup = 130,
}

impl Default for Chattype {
    fn default() -> Self {
        Chattype::Undefined
    }
}

pub const DC_MSG_ID_MARKER1: u32 = 1;
const DC_MSG_ID_DAYMARKER: u32 = 9;
pub const DC_MSG_ID_LAST_SPECIAL: u32 = 9;

/// approx. max. length returned by dc_msg_get_text()
const DC_MAX_GET_TEXT_LEN: usize = 30000;
/// approx. max. length returned by dc_get_msg_info()
const DC_MAX_GET_INFO_LEN: usize = 100000;

pub const DC_CONTACT_ID_UNDEFINED: u32 = 0;
pub const DC_CONTACT_ID_SELF: u32 = 1;
const DC_CONTACT_ID_DEVICE: u32 = 2;
pub const DC_CONTACT_ID_LAST_SPECIAL: u32 = 9;

pub const DC_CREATE_MVBOX: usize = 1;

// Flags for configuring IMAP and SMTP servers.
// These flags are optional
// and may be set together with the username, password etc.
// via dc_set_config() using the key "server_flags".

/// Force OAuth2 authorization. This flag does not skip automatic configuration.
/// Before calling configure() with DC_LP_AUTH_OAUTH2 set,
/// the user has to confirm access at the URL returned by dc_get_oauth2_url().
pub const DC_LP_AUTH_OAUTH2: usize = 0x2;

/// Force NORMAL authorization, this is the default.
/// If this flag is set, automatic configuration is skipped.
const DC_LP_AUTH_NORMAL: usize = 0x4;

/// Connect to IMAP via STARTTLS.
/// If this flag is set, automatic configuration is skipped.
pub const DC_LP_IMAP_SOCKET_STARTTLS: usize = 0x100;

/// Connect to IMAP via SSL.
/// If this flag is set, automatic configuration is skipped.
const DC_LP_IMAP_SOCKET_SSL: usize = 0x200;

/// Connect to IMAP unencrypted, this should not be used.
/// If this flag is set, automatic configuration is skipped.
pub const DC_LP_IMAP_SOCKET_PLAIN: usize = 0x400;

/// Connect to SMTP via STARTTLS.
/// If this flag is set, automatic configuration is skipped.
pub const DC_LP_SMTP_SOCKET_STARTTLS: usize = 0x10000;

/// Connect to SMTP via SSL.
/// If this flag is set, automatic configuration is skipped.
const DC_LP_SMTP_SOCKET_SSL: usize = 0x20000;

/// Connect to SMTP unencrypted, this should not be used.
/// If this flag is set, automatic configuration is skipped.
pub const DC_LP_SMTP_SOCKET_PLAIN: usize = 0x40000;

/// if none of these flags are set, the default is chosen
const DC_LP_AUTH_FLAGS: usize = (DC_LP_AUTH_OAUTH2 | DC_LP_AUTH_NORMAL);
/// if none of these flags are set, the default is chosen
const DC_LP_IMAP_SOCKET_FLAGS: usize =
    (DC_LP_IMAP_SOCKET_STARTTLS | DC_LP_IMAP_SOCKET_SSL | DC_LP_IMAP_SOCKET_PLAIN);
/// if none of these flags are set, the default is chosen
const DC_LP_SMTP_SOCKET_FLAGS: usize =
    (DC_LP_SMTP_SOCKET_STARTTLS | DC_LP_SMTP_SOCKET_SSL | DC_LP_SMTP_SOCKET_PLAIN);

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(i32)]
pub enum Viewtype {
    Unknown = 0,
    /// Text message.
    /// The text of the message is set using dc_msg_set_text()
    /// and retrieved with dc_msg_get_text().
    Text = 10,

    /// Image message.
    /// If the image is an animated GIF, the type DC_MSG_GIF should be used.
    /// File, width and height are set via dc_msg_set_file(), dc_msg_set_dimension
    /// and retrieved via dc_msg_set_file(), dc_msg_set_dimension().
    Image = 20,

    /// Animated GIF message.
    /// File, width and height are set via dc_msg_set_file(), dc_msg_set_dimension()
    /// and retrieved via dc_msg_get_file(), dc_msg_get_width(), dc_msg_get_height().
    Gif = 21,

    /// Message containing an Audio file.
    /// File and duration are set via dc_msg_set_file(), dc_msg_set_duration()
    /// and retrieved via dc_msg_get_file(), dc_msg_get_duration().
    Audio = 40,

    /// A voice message that was directly recorded by the user.
    /// For all other audio messages, the type #DC_MSG_AUDIO should be used.
    /// File and duration are set via dc_msg_set_file(), dc_msg_set_duration()
    /// and retrieved via dc_msg_get_file(), dc_msg_get_duration()
    Voice = 41,

    /// Video messages.
    /// File, width, height and durarion
    /// are set via dc_msg_set_file(), dc_msg_set_dimension(), dc_msg_set_duration()
    /// and retrieved via
    /// dc_msg_get_file(), dc_msg_get_width(),
    /// dc_msg_get_height(), dc_msg_get_duration().
    Video = 50,

    /// Message containing any file, eg. a PDF.
    /// The file is set via dc_msg_set_file()
    /// and retrieved via dc_msg_get_file().
    File = 60,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_display_works_as_expected() {
        assert_eq!(format!("{}", Viewtype::Audio), "Audio");
    }
}

// These constants are used as events
// reported to the callback given to dc_context_new().
// If you do not want to handle an event, it is always safe to return 0,
// so there is no need to add a "case" for every event.

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum Event {
    /// The library-user may write an informational string to the log.
    /// Passed to the callback given to dc_context_new().
    /// This event should not be reported to the end-user using a popup or something like that.
    /// @param data1 0
    /// @param data2 (const char*) Info string in english language.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @return 0
    INFO = 100,

    /// Emitted when SMTP connection is established and login was successful.
    ///
    /// @param data1 0
    /// @param data2 (const char*) Info string in english language.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @return 0
    SMTP_CONNECTED = 101,

    /// Emitted when IMAP connection is established and login was successful.
    ///
    /// @param data1 0
    /// @param data2 (const char*) Info string in english language.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @return 0
    IMAP_CONNECTED = 102,

    /// Emitted when a message was successfully sent to the SMTP server.
    ///
    /// @param data1 0
    /// @param data2 (const char*) Info string in english language.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @return 0
    SMTP_MESSAGE_SENT = 103,

    /// The library-user should write a warning string to the log.
    /// Passed to the callback given to dc_context_new().
    ///
    /// This event should not be reported to the end-user using a popup or something like that.
    ///
    /// @param data1 0
    /// @param data2 (const char*) Warning string in english language.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @return 0
    WARNING = 300,

    /// The library-user should report an error to the end-user.
    /// Passed to the callback given to dc_context_new().
    ///
    /// As most things are asynchronous, things may go wrong at any time and the user
    /// should not be disturbed by a dialog or so.  Instead, use a bubble or so.
    ///
    /// However, for ongoing processes (eg. configure())
    /// or for functions that are expected to fail (eg. dc_continue_key_transfer())
    /// it might be better to delay showing these events until the function has really
    /// failed (returned false). It should be sufficient to report only the _last_ error
    /// in a messasge box then.
    ///
    /// @param data1 0
    /// @param data2 (const char*) Error string, always set, never NULL. Frequent error strings are
    ///     localized using #DC_EVENT_GET_STRING, however, most error strings will be in english language.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @return 0
    ERROR = 400,

    /// An action cannot be performed because there is no network available.
    ///
    /// The library will typically try over after a some time
    /// and when dc_maybe_network() is called.
    ///
    /// Network errors should be reported to users in a non-disturbing way,
    /// however, as network errors may come in a sequence,
    /// it is not useful to raise each an every error to the user.
    /// For this purpose, data1 is set to 1 if the error is probably worth reporting.
    ///
    /// Moreover, if the UI detects that the device is offline,
    /// it is probably more useful to report this to the user
    /// instead of the string from data2.
    ///
    /// @param data1 (int) 1=first/new network error, should be reported the user;
    ///     0=subsequent network error, should be logged only
    /// @param data2 (const char*) Error string, always set, never NULL.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @return 0
    ERROR_NETWORK = 401,

    /// An action cannot be performed because the user is not in the group.
    /// Reported eg. after a call to
    /// dc_set_chat_name(), dc_set_chat_profile_image(),
    /// dc_add_contact_to_chat(), dc_remove_contact_from_chat(),
    /// dc_send_text_msg() or another sending function.
    ///
    /// @param data1 0
    /// @param data2 (const char*) Info string in english language.
    ///     Must not be free()'d or modified
    ///     and is valid only until the callback returns.
    /// @return 0
    ERROR_SELF_NOT_IN_GROUP = 410,

    /// Messages or chats changed.  One or more messages or chats changed for various
    /// reasons in the database:
    /// - Messages sent, received or removed
    /// - Chats created, deleted or archived
    /// - A draft has been set
    ///
    /// @param data1 (int) chat_id for single added messages
    /// @param data2 (int) msg_id for single added messages
    /// @return 0
    MSGS_CHANGED = 2000,

    /// There is a fresh message. Typically, the user will show an notification
    /// when receiving this message.
    ///
    /// There is no extra #DC_EVENT_MSGS_CHANGED event send together with this event.
    ///
    /// @param data1 (int) chat_id
    /// @param data2 (int) msg_id
    /// @return 0
    INCOMING_MSG = 2005,

    /// A single message is sent successfully. State changed from  DC_STATE_OUT_PENDING to
    /// DC_STATE_OUT_DELIVERED, see dc_msg_get_state().
    ///
    /// @param data1 (int) chat_id
    /// @param data2 (int) msg_id
    /// @return 0
    MSG_DELIVERED = 2010,

    /// A single message could not be sent. State changed from DC_STATE_OUT_PENDING or DC_STATE_OUT_DELIVERED to
    /// DC_STATE_OUT_FAILED, see dc_msg_get_state().
    ///
    /// @param data1 (int) chat_id
    /// @param data2 (int) msg_id
    /// @return 0
    MSG_FAILED = 2012,

    /// A single message is read by the receiver. State changed from DC_STATE_OUT_DELIVERED to
    /// DC_STATE_OUT_MDN_RCVD, see dc_msg_get_state().
    ///
    /// @param data1 (int) chat_id
    /// @param data2 (int) msg_id
    /// @return 0
    MSG_READ = 2015,

    /// Chat changed.  The name or the image of a chat group was changed or members were added or removed.
    /// Or the verify state of a chat has changed.
    /// See dc_set_chat_name(), dc_set_chat_profile_image(), dc_add_contact_to_chat()
    /// and dc_remove_contact_from_chat().
    ///
    /// @param data1 (int) chat_id
    /// @param data2 0
    /// @return 0
    CHAT_MODIFIED = 2020,

    /// Contact(s) created, renamed, blocked or deleted.
    ///
    /// @param data1 (int) If not 0, this is the contact_id of an added contact that should be selected.
    /// @param data2 0
    /// @return 0
    CONTACTS_CHANGED = 2030,

    /// Location of one or more contact has changed.
    ///
    /// @param data1 (int) contact_id of the contact for which the location has changed.
    ///     If the locations of several contacts have been changed,
    ///     eg. after calling dc_delete_all_locations(), this parameter is set to 0.
    /// @param data2 0
    /// @return 0
    LOCATION_CHANGED = 2035,

    /// Inform about the configuration progress started by configure().
    ///
    /// @param data1 (int) 0=error, 1-999=progress in permille, 1000=success and done
    /// @param data2 0
    /// @return 0
    CONFIGURE_PROGRESS = 2041,

    /// Inform about the import/export progress started by dc_imex().
    ///
    /// @param data1 (int) 0=error, 1-999=progress in permille, 1000=success and done
    /// @param data2 0
    /// @return 0
    IMEX_PROGRESS = 2051,

    /// A file has been exported. A file has been written by dc_imex().
    /// This event may be sent multiple times by a single call to dc_imex().
    ///
    /// A typical purpose for a handler of this event may be to make the file public to some system
    /// services.
    ///
    /// @param data1 (const char*) Path and file name.
    ///     Must not be free()'d or modified and is valid only until the callback returns.
    /// @param data2 0
    /// @return 0
    IMEX_FILE_WRITTEN = 2052,

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
    /// @return 0
    SECUREJOIN_INVITER_PROGRESS = 2060,

    /// Progress information of a secure-join handshake from the view of the joiner
    /// (Bob, the person who scans the QR code).
    /// The events are typically sent while dc_join_securejoin(), which
    /// may take some time, is executed.
    /// @param data1 (int) ID of the inviting contact.
    /// @param data2 (int) Progress as:
    ///     400=vg-/vc-request-with-auth sent, typically shown as "alice@addr verified, introducing myself."
    ///     (Bob has verified alice and waits until Alice does the same for him)
    /// @return 0
    SECUREJOIN_JOINER_PROGRESS = 2061,

    // the following events are functions that should be provided by the frontends
    /// Requeste a localized string from the frontend.
    /// @param data1 (int) ID of the string to request, one of the DC_STR_/// constants.
    /// @param data2 (int) The count. If the requested string contains a placeholder for a numeric value,
    ///     the ui may use this value to return different strings on different plural forms.
    /// @return (const char*) Null-terminated UTF-8 string.
    ///     The string will be free()'d by the core,
    ///     so it must be allocated using malloc() or a compatible function.
    ///     Return 0 if the ui cannot provide the requested string
    ///     the core will use a default string in english language then.
    GET_STRING = 2091,
}

const DC_EVENT_FILE_COPIED: usize = 2055; // deprecated;
const DC_EVENT_IS_OFFLINE: usize = 2081; // deprecated;
const DC_ERROR_SEE_STRING: usize = 0; // deprecated;
const DC_ERROR_SELF_NOT_IN_GROUP: usize = 1; // deprecated;
const DC_STR_SELFNOTINGRP: usize = 21; // deprecated;

/// Values for dc_get|set_config("show_emails")
const DC_SHOW_EMAILS_OFF: usize = 0;
const DC_SHOW_EMAILS_ACCEPTED_CONTACTS: usize = 1;
const DC_SHOW_EMAILS_ALL: usize = 2;

// TODO: Strings need some doumentation about used placeholders.
// These constants are used to request strings using #DC_EVENT_GET_STRING.

const DC_STR_NOMESSAGES: usize = 1;
const DC_STR_SELF: usize = 2;
const DC_STR_DRAFT: usize = 3;
const DC_STR_MEMBER: usize = 4;
const DC_STR_CONTACT: usize = 6;
const DC_STR_VOICEMESSAGE: usize = 7;
const DC_STR_DEADDROP: usize = 8;
const DC_STR_IMAGE: usize = 9;
const DC_STR_VIDEO: usize = 10;
const DC_STR_AUDIO: usize = 11;
const DC_STR_FILE: usize = 12;
const DC_STR_STATUSLINE: usize = 13;
const DC_STR_NEWGROUPDRAFT: usize = 14;
const DC_STR_MSGGRPNAME: usize = 15;
const DC_STR_MSGGRPIMGCHANGED: usize = 16;
const DC_STR_MSGADDMEMBER: usize = 17;
const DC_STR_MSGDELMEMBER: usize = 18;
const DC_STR_MSGGROUPLEFT: usize = 19;
const DC_STR_GIF: usize = 23;
const DC_STR_ENCRYPTEDMSG: usize = 24;
const DC_STR_E2E_AVAILABLE: usize = 25;
const DC_STR_ENCR_TRANSP: usize = 27;
const DC_STR_ENCR_NONE: usize = 28;
const DC_STR_CANTDECRYPT_MSG_BODY: usize = 29;
const DC_STR_FINGERPRINTS: usize = 30;
const DC_STR_READRCPT: usize = 31;
const DC_STR_READRCPT_MAILBODY: usize = 32;
const DC_STR_MSGGRPIMGDELETED: usize = 33;
const DC_STR_E2E_PREFERRED: usize = 34;
const DC_STR_CONTACT_VERIFIED: usize = 35;
const DC_STR_CONTACT_NOT_VERIFIED: usize = 36;
const DC_STR_CONTACT_SETUP_CHANGED: usize = 37;
const DC_STR_ARCHIVEDCHATS: usize = 40;
const DC_STR_STARREDMSGS: usize = 41;
const DC_STR_AC_SETUP_MSG_SUBJECT: usize = 42;
const DC_STR_AC_SETUP_MSG_BODY: usize = 43;
const DC_STR_SELFTALK_SUBTITLE: usize = 50;
const DC_STR_CANNOT_LOGIN: usize = 60;
const DC_STR_SERVER_RESPONSE: usize = 61;
const DC_STR_MSGACTIONBYUSER: usize = 62;
const DC_STR_MSGACTIONBYME: usize = 63;
const DC_STR_MSGLOCATIONENABLED: usize = 64;
const DC_STR_MSGLOCATIONDISABLED: usize = 65;
const DC_STR_LOCATION: usize = 66;
const DC_STR_COUNT: usize = 66;

pub const DC_JOB_DELETE_MSG_ON_IMAP: i32 = 110;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum KeyType {
    Public = 0,
    Private = 1,
}

pub const DC_CMD_GROUPNAME_CHANGED: libc::c_int = 2;
pub const DC_CMD_GROUPIMAGE_CHANGED: libc::c_int = 3;
pub const DC_CMD_MEMBER_ADDED_TO_GROUP: libc::c_int = 4;
pub const DC_CMD_MEMBER_REMOVED_FROM_GROUP: libc::c_int = 5;
pub const DC_CMD_AUTOCRYPT_SETUP_MESSAGE: libc::c_int = 6;
const DC_CMD_SECUREJOIN_MESSAGE: libc::c_int = 7;
pub const DC_CMD_LOCATION_STREAMING_ENABLED: libc::c_int = 8;
const DC_CMD_LOCATION_ONLY: libc::c_int = 9;
