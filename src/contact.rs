//! Contacts module

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, ensure, Context as _, Result};
use async_channel::{self as channel, Receiver, Sender};
use deltachat_derive::{FromSql, ToSql};
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tokio::task;
use tokio::time::{timeout, Duration};

use crate::aheader::EncryptPreference;
use crate::chat::ChatId;
use crate::color::str_to_color;
use crate::config::Config;
use crate::constants::{Blocked, Chattype, DC_GCL_ADD_SELF, DC_GCL_VERIFIED_ONLY};
use crate::context::Context;
use crate::events::EventType;
use crate::key::{load_self_public_key, DcKey};
use crate::login_param::LoginParam;
use crate::message::MessageState;
use crate::mimeparser::AvatarAction;
use crate::param::{Param, Params};
use crate::peerstate::{Peerstate, PeerstateVerifiedStatus};
use crate::sql::{self, params_iter};
use crate::tools::{
    duration_to_str, get_abs_path, improve_single_line_input, strip_rtlo_characters, time,
    EmailAddress,
};
use crate::{chat, stock_str};

/// Time during which a contact is considered as seen recently.
const SEEN_RECENTLY_SECONDS: i64 = 600;

/// Valid contact address.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContactAddress<'a>(&'a str);

impl Deref for ContactAddress<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl AsRef<str> for ContactAddress<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl fmt::Display for ContactAddress<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> ContactAddress<'a> {
    /// Constructs a new contact address from string,
    /// normalizing and validating it.
    pub fn new(s: &'a str) -> Result<Self> {
        let addr = addr_normalize(s);
        if !may_be_valid_addr(addr) {
            bail!("invalid address {:?}", s);
        }
        Ok(Self(addr))
    }
}

/// Allow converting [`ContactAddress`] to an SQLite type.
impl rusqlite::types::ToSql for ContactAddress<'_> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let val = rusqlite::types::Value::Text(self.0.to_string());
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

/// Contact ID, including reserved IDs.
///
/// Some contact IDs are reserved to identify special contacts.  This
/// type can represent both the special as well as normal contacts.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ContactId(u32);

impl ContactId {
    /// Undefined contact. Used as a placeholder for trashed messages.
    pub const UNDEFINED: ContactId = ContactId::new(0);

    /// The owner of the account.
    ///
    /// The email-address is set by `set_config` using "addr".
    pub const SELF: ContactId = ContactId::new(1);

    /// ID of the contact for info messages.
    pub const INFO: ContactId = ContactId::new(2);

    /// ID of the contact for device messages.
    pub const DEVICE: ContactId = ContactId::new(5);
    pub(crate) const LAST_SPECIAL: ContactId = ContactId::new(9);

    /// Address to go with [`ContactId::DEVICE`].
    ///
    /// This is used by APIs which need to return an email address for this contact.
    pub const DEVICE_ADDR: &'static str = "device@localhost";

    /// Creates a new [`ContactId`].
    pub const fn new(id: u32) -> ContactId {
        ContactId(id)
    }

    /// Whether this is a special [`ContactId`].
    ///
    /// Some [`ContactId`]s are reserved for special contacts like [`ContactId::SELF`],
    /// [`ContactId::INFO`] and [`ContactId::DEVICE`].  This function indicates whether this
    /// [`ContactId`] is any of the reserved special [`ContactId`]s (`true`) or whether it
    /// is the [`ContactId`] of a real contact (`false`).
    pub fn is_special(&self) -> bool {
        self.0 <= Self::LAST_SPECIAL.0
    }

    /// Numerical representation of the [`ContactId`].
    ///
    /// Each contact ID has a unique numerical representation which is used in the database
    /// (via [`rusqlite::ToSql`]) and also for FFI purposes.  In Rust code you should never
    /// need to use this directly.
    pub const fn to_u32(&self) -> u32 {
        self.0
    }

    /// Mark contact as bot.
    pub async fn mark_bot(&self, context: &Context, is_bot: bool) -> Result<()> {
        context
            .sql
            .execute("UPDATE contacts SET is_bot=? WHERE id=?;", (self.0, is_bot))
            .await?;
        Ok(())
    }
}

impl fmt::Display for ContactId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if *self == ContactId::UNDEFINED {
            write!(f, "Contact#Undefined")
        } else if *self == ContactId::SELF {
            write!(f, "Contact#Self")
        } else if *self == ContactId::INFO {
            write!(f, "Contact#Info")
        } else if *self == ContactId::DEVICE {
            write!(f, "Contact#Device")
        } else if self.is_special() {
            write!(f, "Contact#Special{}", self.0)
        } else {
            write!(f, "Contact#{}", self.0)
        }
    }
}

/// Allow converting [`ContactId`] to an SQLite type.
impl rusqlite::types::ToSql for ContactId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let val = rusqlite::types::Value::Integer(i64::from(self.0));
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

/// Allow converting an SQLite integer directly into [`ContactId`].
impl rusqlite::types::FromSql for ContactId {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).and_then(|val| {
            val.try_into()
                .map(ContactId::new)
                .map_err(|_| rusqlite::types::FromSqlError::OutOfRange(val))
        })
    }
}

/// An object representing a single contact in memory.
///
/// The contact object is not updated.
/// If you want an update, you have to recreate the object.
///
/// The library makes sure
/// only to use names _authorized_ by the contact in `To:` or `Cc:`.
/// *Given-names* as "Daddy" or "Honey" are not used there.
/// For this purpose, internally, two names are tracked -
/// authorized name and given name.
/// By default, these names are equal, but functions working with contact names
/// only affect the given name.
#[derive(Debug)]
pub struct Contact {
    /// The contact ID.
    pub id: ContactId,

    /// Contact name. It is recommended to use `Contact::get_name`,
    /// `Contact::get_display_name` or `Contact::get_name_n_addr` to access this field.
    /// May be empty, initially set to `authname`.
    name: String,

    /// Name authorized by the contact himself. Only this name may be spread to others,
    /// e.g. in To:-lists. May be empty. It is recommended to use `Contact::get_authname`,
    /// to access this field.
    authname: String,

    /// E-Mail-Address of the contact. It is recommended to use `Contact::get_addr` to access this field.
    addr: String,

    /// Blocked state. Use contact_is_blocked to access this field.
    pub blocked: bool,

    /// Time when the contact was seen last time, Unix time in seconds.
    last_seen: i64,

    /// The origin/source of the contact.
    pub origin: Origin,

    /// Parameters as Param::ProfileImage
    pub param: Params,

    /// Last seen message signature for this contact, to be displayed in the profile.
    status: String,

    /// If the contact is a bot.
    is_bot: bool,
}

/// Possible origins of a contact.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
)]
#[repr(u32)]
pub enum Origin {
    /// Unknown origin. Can be used as a minimum origin to specify that the caller does not care
    /// about origin of the contact.
    #[default]
    Unknown = 0,

    /// The contact is a mailing list address, needed to unblock mailing lists
    MailinglistAddress = 0x2,

    /// Hidden on purpose, e.g. addresses with the word "noreply" in it
    Hidden = 0x8,

    /// From: of incoming messages of unknown sender
    IncomingUnknownFrom = 0x10,

    /// Cc: of incoming messages of unknown sender
    IncomingUnknownCc = 0x20,

    /// To: of incoming messages of unknown sender
    IncomingUnknownTo = 0x40,

    /// address scanned but not verified
    UnhandledQrScan = 0x80,

    /// Reply-To: of incoming message of known sender
    /// Contacts with at least this origin value are shown in the contact list.
    IncomingReplyTo = 0x100,

    /// Cc: of incoming message of known sender
    IncomingCc = 0x200,

    /// additional To:'s of incoming message of known sender
    IncomingTo = 0x400,

    /// a chat was manually created for this user, but no message yet sent
    CreateChat = 0x800,

    /// message sent by us
    OutgoingBcc = 0x1000,

    /// message sent by us
    OutgoingCc = 0x2000,

    /// message sent by us
    OutgoingTo = 0x4000,

    /// internal use
    Internal = 0x40000,

    /// address is in our address book
    AddressBook = 0x80000,

    /// set on Alice's side for contacts like Bob that have scanned the QR code offered by her. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling contact_is_verified() !
    SecurejoinInvited = 0x0100_0000,

    /// set on Bob's side for contacts scanned and verified from a QR code. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling contact_is_verified() !
    SecurejoinJoined = 0x0200_0000,

    /// contact added manually by create_contact(), this should be the largest origin as otherwise the user cannot modify the names
    ManuallyCreated = 0x0400_0000,
}

impl Origin {
    /// Contacts that are known, i. e. they came in via accepted contacts or
    /// themselves an accepted contact. Known contacts are shown in the
    /// contact list when one creates a chat and wants to add members etc.
    pub fn is_known(self) -> bool {
        self >= Origin::IncomingReplyTo
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Modifier {
    None,
    Modified,
    Created,
}

/// Verification status of the contact.
#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
pub enum VerifiedStatus {
    /// Contact is not verified.
    Unverified = 0,
    /// SELF has verified the fingerprint of a contact. Currently unused.
    Verified = 1,
    /// SELF and contact have verified their fingerprints in both directions; in the UI typically checkmarks are shown.
    BidirectVerified = 2,
}

impl Default for VerifiedStatus {
    fn default() -> Self {
        Self::Unverified
    }
}

impl Contact {
    /// Loads a single contact object from the database.
    ///
    /// Returns an error if the contact does not exist.
    ///
    /// For contact ContactId::SELF (1), the function returns sth.
    /// like "Me" in the selected language and the email address
    /// defined by set_config().
    ///
    /// For contact ContactId::DEVICE, the function overrides
    /// the contact name and status with localized address.
    pub async fn get_by_id(context: &Context, contact_id: ContactId) -> Result<Self> {
        let contact = Self::get_by_id_optional(context, contact_id)
            .await?
            .with_context(|| format!("contact {contact_id} not found"))?;
        Ok(contact)
    }

    /// Loads a single contact object from the database.
    ///
    /// Similar to [`Contact::get_by_id()`] but returns `None` if the contact does not exist.
    pub async fn get_by_id_optional(
        context: &Context,
        contact_id: ContactId,
    ) -> Result<Option<Self>> {
        if let Some(mut contact) = context
            .sql
            .query_row_optional(
                "SELECT c.name, c.addr, c.origin, c.blocked, c.last_seen,
                c.authname, c.param, c.status, c.is_bot
               FROM contacts c
              WHERE c.id=?;",
                (contact_id,),
                |row| {
                    let name: String = row.get(0)?;
                    let addr: String = row.get(1)?;
                    let origin: Origin = row.get(2)?;
                    let blocked: Option<bool> = row.get(3)?;
                    let last_seen: i64 = row.get(4)?;
                    let authname: String = row.get(5)?;
                    let param: String = row.get(6)?;
                    let status: Option<String> = row.get(7)?;
                    let is_bot: bool = row.get(8)?;
                    let contact = Self {
                        id: contact_id,
                        name,
                        authname,
                        addr,
                        blocked: blocked.unwrap_or_default(),
                        last_seen,
                        origin,
                        param: param.parse().unwrap_or_default(),
                        status: status.unwrap_or_default(),
                        is_bot,
                    };
                    Ok(contact)
                },
            )
            .await?
        {
            if contact_id == ContactId::SELF {
                contact.name = stock_str::self_msg(context).await;
                contact.addr = context
                    .get_config(Config::ConfiguredAddr)
                    .await?
                    .unwrap_or_default();
                contact.status = context
                    .get_config(Config::Selfstatus)
                    .await?
                    .unwrap_or_default();
            } else if contact_id == ContactId::DEVICE {
                contact.name = stock_str::device_messages(context).await;
                contact.addr = ContactId::DEVICE_ADDR.to_string();
                contact.status = stock_str::device_messages_hint(context).await;
            }
            Ok(Some(contact))
        } else {
            Ok(None)
        }
    }

    /// Returns `true` if this contact is blocked.
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Returns last seen timestamp.
    pub fn last_seen(&self) -> i64 {
        self.last_seen
    }

    /// Returns `true` if this contact was seen recently.
    pub fn was_seen_recently(&self) -> bool {
        time() - self.last_seen <= SEEN_RECENTLY_SECONDS
    }

    /// Check if a contact is blocked.
    pub async fn is_blocked_load(context: &Context, id: ContactId) -> Result<bool> {
        let blocked = context
            .sql
            .query_row("SELECT blocked FROM contacts WHERE id=?", (id,), |row| {
                let blocked: bool = row.get(0)?;
                Ok(blocked)
            })
            .await?;
        Ok(blocked)
    }

    /// Block the given contact.
    pub async fn block(context: &Context, id: ContactId) -> Result<()> {
        set_block_contact(context, id, true).await
    }

    /// Unblock the given contact.
    pub async fn unblock(context: &Context, id: ContactId) -> Result<()> {
        set_block_contact(context, id, false).await
    }

    /// Add a single contact as a result of an _explicit_ user action.
    ///
    /// We assume, the contact name, if any, is entered by the user and is used "as is" therefore,
    /// normalize() is *not* called for the name. If the contact is blocked, it is unblocked.
    ///
    /// To add a number of contacts, see `add_address_book()` which is much faster for adding
    /// a bunch of addresses.
    ///
    /// May result in a `#DC_EVENT_CONTACTS_CHANGED` event.
    pub async fn create(context: &Context, name: &str, addr: &str) -> Result<ContactId> {
        let name = improve_single_line_input(name);

        let (name, addr) = sanitize_name_and_addr(&name, addr);
        let addr = ContactAddress::new(&addr)?;

        let (contact_id, sth_modified) =
            Contact::add_or_lookup(context, &name, addr, Origin::ManuallyCreated)
                .await
                .context("add_or_lookup")?;
        let blocked = Contact::is_blocked_load(context, contact_id).await?;
        match sth_modified {
            Modifier::None => {}
            Modifier::Modified | Modifier::Created => {
                context.emit_event(EventType::ContactsChanged(Some(contact_id)))
            }
        }
        if blocked {
            Contact::unblock(context, contact_id).await?;
        }

        Ok(contact_id)
    }

    /// Mark messages from a contact as noticed.
    pub async fn mark_noticed(context: &Context, id: ContactId) -> Result<()> {
        context
            .sql
            .execute(
                "UPDATE msgs SET state=? WHERE from_id=? AND state=?;",
                (MessageState::InNoticed, id, MessageState::InFresh),
            )
            .await?;
        Ok(())
    }

    /// Mark bot.
    pub async fn mark_bot(context: &Context, id: ContactId) -> Result<()> {
        context
            .sql
            .execute("UPDATE contacts SET is_bot=1 WHERE id=?;", (id,))
            .await?;
        Ok(())
    }

    /// Returns whether contact is a bot.
    pub fn is_bot(&self) -> bool {
        self.is_bot
    }

    /// Check if an e-mail address belongs to a known and unblocked contact.
    ///
    /// Known and unblocked contacts will be returned by `get_contacts()`.
    ///
    /// To validate an e-mail address independently of the contact database
    /// use `may_be_valid_addr()`.
    pub async fn lookup_id_by_addr(
        context: &Context,
        addr: &str,
        min_origin: Origin,
    ) -> Result<Option<ContactId>> {
        if addr.is_empty() {
            bail!("lookup_id_by_addr: empty address");
        }

        let addr_normalized = addr_normalize(addr);

        if context.is_self_addr(addr_normalized).await? {
            return Ok(Some(ContactId::SELF));
        }

        let id = context
            .sql
            .query_get_value(
                "SELECT id FROM contacts \
            WHERE addr=?1 COLLATE NOCASE \
            AND id>?2 AND origin>=?3 AND blocked=0;",
                (&addr_normalized, ContactId::LAST_SPECIAL, min_origin as u32),
            )
            .await?;
        Ok(id)
    }

    /// Lookup a contact and create it if it does not exist yet.
    /// The contact is identified by the email-address, a name and an "origin" can be given.
    ///
    /// The "origin" is where the address comes from -
    /// from-header, cc-header, addressbook, qr, manual-edit etc.
    /// In general, "better" origins overwrite the names of "worse" origins -
    /// Eg. if we got a name in cc-header and later in from-header, the name will change -
    /// this does not happen the other way round.
    ///
    /// The "best" origin are manually created contacts -
    /// names given manually can only be overwritten by further manual edits
    /// (until they are set empty again or reset to the name seen in the From-header).
    ///
    /// These manually edited names are _never_ used for sending on the wire -
    /// this should avoid sending sth. as "Mama" or "Daddy" to some 3rd party.
    /// Instead, for the wire, we use so called "authnames"
    /// that can only be set and updated by a From-header.
    ///
    /// The different names used in the function are:
    /// - "name": name passed as function argument, belonging to the given origin
    /// - "row_name": current name used in the database, typically set to "name"
    /// - "row_authname": name as authorized from a contact, set only through a From-header
    /// Depending on the origin, both, "row_name" and "row_authname" are updated from "name".
    ///
    /// Returns the contact_id and a `Modifier` value indicating if a modification occurred.
    pub(crate) async fn add_or_lookup(
        context: &Context,
        name: &str,
        addr: ContactAddress<'_>,
        mut origin: Origin,
    ) -> Result<(ContactId, Modifier)> {
        let mut sth_modified = Modifier::None;

        ensure!(!addr.is_empty(), "Can not add_or_lookup empty address");
        ensure!(origin != Origin::Unknown, "Missing valid origin");

        if context.is_self_addr(&addr).await? {
            return Ok((ContactId::SELF, sth_modified));
        }

        let mut name = strip_rtlo_characters(name);
        #[allow(clippy::collapsible_if)]
        if origin <= Origin::OutgoingTo {
            // The user may accidentally have written to a "noreply" address with another MUA:
            if addr.contains("noreply")
                || addr.contains("no-reply")
                || addr.starts_with("notifications@")
                // Filter out use-once addresses (like reply+AEJDGPOECLAP...@reply.github.com):
                || (addr.len() > 50 && addr.contains('+'))
            {
                info!(context, "hiding contact {}", addr);
                origin = Origin::Hidden;
                // For these kind of email addresses, sender and address often don't belong together
                // (like hocuri <notifications@github.com>). In this example, hocuri shouldn't
                // be saved as the displayname for notifications@github.com.
                name = "".to_string();
            }
        }

        // If the origin indicates that user entered the contact manually, from the address book or
        // from the QR-code scan (potentially from the address book of their other phone), then name
        // should go into the "name" column and never into "authname" column, to avoid leaking it
        // into the network.
        let manual = matches!(
            origin,
            Origin::ManuallyCreated | Origin::AddressBook | Origin::UnhandledQrScan
        );

        let mut update_addr = false;

        let row_id = context.sql.transaction(|transaction| {
            let row = transaction.query_row(
                "SELECT id, name, addr, origin, authname
                 FROM contacts WHERE addr=? COLLATE NOCASE",
                 [addr.to_string()],
                |row| {
                    let row_id: isize = row.get(0)?;
                    let row_name: String = row.get(1)?;
                    let row_addr: String = row.get(2)?;
                    let row_origin: Origin = row.get(3)?;
                    let row_authname: String = row.get(4)?;

                    Ok((row_id, row_name, row_addr, row_origin, row_authname))
                }).optional()?;

            let row_id;
            if let Some((id, row_name, row_addr, row_origin, row_authname)) = row {
                let update_name = manual && name != row_name;
                let update_authname = !manual
                    && name != row_authname
                    && !name.is_empty()
                    && (origin >= row_origin
                        || origin == Origin::IncomingUnknownFrom
                        || row_authname.is_empty());

                row_id = u32::try_from(id)?;
                if origin >= row_origin && addr.as_ref() != row_addr {
                    update_addr = true;
                }
                if update_name || update_authname || update_addr || origin > row_origin {
                    let new_name = if update_name {
                        name.to_string()
                    } else {
                        row_name
                    };

                    transaction
                        .execute(
                            "UPDATE contacts SET name=?, addr=?, origin=?, authname=? WHERE id=?;",
                            (
                                new_name,
                                if update_addr {
                                    addr.to_string()
                                } else {
                                    row_addr
                                },
                                if origin > row_origin {
                                    origin
                                } else {
                                    row_origin
                                },
                                if update_authname {
                                    name.to_string()
                                } else {
                                    row_authname
                                },
                                row_id
                            ),
                        )?;

                    if update_name || update_authname {
                        // Update the contact name also if it is used as a group name.
                        // This is one of the few duplicated data, however, getting the chat list is easier this way.
                        let chat_id: Option<ChatId> = transaction.query_row(
                            "SELECT id FROM chats WHERE type=? AND id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?)",
                            (Chattype::Single, isize::try_from(row_id)?),
                            |row| {
                                let chat_id: ChatId = row.get(0)?;
                                Ok(chat_id)
                            }
                        ).optional()?;

                        if let Some(chat_id) = chat_id {
                            let contact_id = ContactId::new(row_id);
                            let (addr, name, authname) =
                                transaction.query_row(
                                    "SELECT addr, name, authname
                                     FROM contacts
                                     WHERE id=?",
                                     (contact_id,),
                                |row| {
                                    let addr: String = row.get(0)?;
                                    let name: String = row.get(1)?;
                                    let authname: String = row.get(2)?;
                                    Ok((addr, name, authname))
                                })?;

                            let chat_name = if !name.is_empty() {
                                name
                            } else if !authname.is_empty() {
                                authname
                            } else {
                                addr
                            };

                            let count = transaction.execute(
                                    "UPDATE chats SET name=?1 WHERE id=?2 AND name!=?1",
                                    (chat_name, chat_id))?;

                            if count > 0 {
                                // Chat name updated
                                context.emit_event(EventType::ChatModified(chat_id));
                            }
                        }
                    }
                    sth_modified = Modifier::Modified;
                }
            } else {
                let update_name = manual;
                let update_authname = !manual;

                transaction
                    .execute(
                        "INSERT INTO contacts (name, addr, origin, authname)
                         VALUES (?, ?, ?, ?);",
                         (
                            if update_name {
                                name.to_string()
                            } else {
                                "".to_string()
                            },
                            addr,
                            origin,
                            if update_authname {
                                name.to_string()
                            } else {
                                "".to_string()
                            }
                        ),
                    )?;

                sth_modified = Modifier::Created;
                row_id = u32::try_from(transaction.last_insert_rowid())?;
                info!(context, "added contact id={} addr={}", row_id, &addr);
            }
            Ok(row_id)
        }).await?;

        Ok((ContactId::new(row_id), sth_modified))
    }

    /// Add a number of contacts.
    ///
    /// Typically used to add the whole address book from the OS. As names here are typically not
    /// well formatted, we call `normalize()` for each name given.
    ///
    /// No email-address is added twice.
    /// Trying to add email-addresses that are already in the contact list,
    /// results in updating the name unless the name was changed manually by the user.
    /// If any email-address or any name is really updated,
    /// the event `DC_EVENT_CONTACTS_CHANGED` is sent.
    ///
    /// To add a single contact entered by the user, you should prefer `Contact::create`,
    /// however, for adding a bunch of addresses, this function is much faster.
    ///
    /// The `addr_book` is a multiline string in the format `Name one\nAddress one\nName two\nAddress two`.
    ///
    /// Returns the number of modified contacts.
    pub async fn add_address_book(context: &Context, addr_book: &str) -> Result<usize> {
        let mut modify_cnt = 0;

        for (name, addr) in split_address_book(addr_book) {
            let (name, addr) = sanitize_name_and_addr(name, addr);
            let name = normalize_name(&name);
            match ContactAddress::new(&addr) {
                Ok(addr) => {
                    match Contact::add_or_lookup(context, &name, addr, Origin::AddressBook).await {
                        Ok((_, modified)) => {
                            if modified != Modifier::None {
                                modify_cnt += 1
                            }
                        }
                        Err(err) => {
                            warn!(
                                context,
                                "Failed to add address {} from address book: {}", addr, err
                            );
                        }
                    }
                }
                Err(err) => {
                    warn!(context, "{:#}.", err);
                }
            }
        }
        if modify_cnt > 0 {
            context.emit_event(EventType::ContactsChanged(None));
        }

        Ok(modify_cnt)
    }

    /// Returns known and unblocked contacts.
    ///
    /// To get information about a single contact, see get_contact().
    ///
    /// `listflags` is a combination of flags:
    /// - if the flag DC_GCL_ADD_SELF is set, SELF is added to the list unless filtered by other parameters
    /// - if the flag DC_GCL_VERIFIED_ONLY is set, only verified contacts are returned.
    ///   if DC_GCL_VERIFIED_ONLY is not set, verified and unverified contacts are returned.
    /// `query` is a string to filter the list.
    pub async fn get_all(
        context: &Context,
        listflags: u32,
        query: Option<&str>,
    ) -> Result<Vec<ContactId>> {
        let self_addrs = context.get_all_self_addrs().await?;
        let mut add_self = false;
        let mut ret = Vec::new();
        let flag_verified_only = (listflags & DC_GCL_VERIFIED_ONLY) != 0;
        let flag_add_self = (listflags & DC_GCL_ADD_SELF) != 0;

        if flag_verified_only || query.is_some() {
            let s3str_like_cmd = format!("%{}%", query.unwrap_or(""));
            context
                .sql
                .query_map(
                    &format!(
                        "SELECT c.id FROM contacts c \
                 LEFT JOIN acpeerstates ps ON c.addr=ps.addr  \
                 WHERE c.addr NOT IN ({})
                 AND c.id>? \
                 AND c.origin>=? \
                 AND c.blocked=0 \
                 AND (iif(c.name='',c.authname,c.name) LIKE ? OR c.addr LIKE ?) \
                 AND (1=? OR LENGTH(ps.verified_key_fingerprint)!=0)  \
                 ORDER BY c.last_seen DESC, c.id DESC;",
                        sql::repeat_vars(self_addrs.len())
                    ),
                    rusqlite::params_from_iter(params_iter(&self_addrs).chain(params_slice![
                        ContactId::LAST_SPECIAL,
                        Origin::IncomingReplyTo,
                        s3str_like_cmd,
                        s3str_like_cmd,
                        if flag_verified_only { 0i32 } else { 1i32 }
                    ])),
                    |row| row.get::<_, ContactId>(0),
                    |ids| {
                        for id in ids {
                            ret.push(id?);
                        }
                        Ok(())
                    },
                )
                .await?;

            if let Some(query) = query {
                let self_addr = context
                    .get_config(Config::ConfiguredAddr)
                    .await?
                    .unwrap_or_default();
                let self_name = context
                    .get_config(Config::Displayname)
                    .await?
                    .unwrap_or_default();
                let self_name2 = stock_str::self_msg(context);

                if self_addr.contains(query)
                    || self_name.contains(query)
                    || self_name2.await.contains(query)
                {
                    add_self = true;
                }
            } else {
                add_self = true;
            }
        } else {
            add_self = true;

            context
                .sql
                .query_map(
                    &format!(
                        "SELECT id FROM contacts
                 WHERE addr NOT IN ({})
                 AND id>?
                 AND origin>=?
                 AND blocked=0
                 ORDER BY last_seen DESC, id DESC;",
                        sql::repeat_vars(self_addrs.len())
                    ),
                    rusqlite::params_from_iter(params_iter(&self_addrs).chain(params_slice![
                        ContactId::LAST_SPECIAL,
                        Origin::IncomingReplyTo
                    ])),
                    |row| row.get::<_, ContactId>(0),
                    |ids| {
                        for id in ids {
                            ret.push(id?);
                        }
                        Ok(())
                    },
                )
                .await?;
        }

        if flag_add_self && add_self {
            ret.push(ContactId::SELF);
        }

        Ok(ret)
    }

    /// Adds blocked mailinglists as contacts
    /// to allow unblocking them as if they are contacts
    /// (this way, only one unblock-ffi is needed and only one set of ui-functions,
    /// from the users perspective,
    /// there is not much difference in an email- and a mailinglist-address)
    async fn update_blocked_mailinglist_contacts(context: &Context) -> Result<()> {
        context
            .sql
            .transaction(move |transaction| {
                let mut stmt = transaction
                    .prepare("SELECT name, grpid FROM chats WHERE type=? AND blocked=?")?;
                let rows = stmt.query_map((Chattype::Mailinglist, Blocked::Yes), |row| {
                    let name: String = row.get(0)?;
                    let grpid: String = row.get(1)?;
                    Ok((name, grpid))
                })?;
                let blocked_mailinglists = rows.collect::<std::result::Result<Vec<_>, _>>()?;
                for (name, grpid) in blocked_mailinglists {
                    let count = transaction.query_row(
                        "SELECT COUNT(id) FROM contacts WHERE addr=?",
                        [&grpid],
                        |row| {
                            let count: isize = row.get(0)?;
                            Ok(count)
                        },
                    )?;
                    if count == 0 {
                        transaction.execute("INSERT INTO contacts (addr) VALUES (?)", [&grpid])?;
                    }

                    // Always do an update in case the blocking is reset or name is changed.
                    transaction.execute(
                        "UPDATE contacts SET name=?, origin=?, blocked=1 WHERE addr=?",
                        (&name, Origin::MailinglistAddress, &grpid),
                    )?;
                }
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Returns number of blocked contacts.
    pub async fn get_blocked_cnt(context: &Context) -> Result<usize> {
        let count = context
            .sql
            .count(
                "SELECT COUNT(*) FROM contacts WHERE id>? AND blocked!=0",
                (ContactId::LAST_SPECIAL,),
            )
            .await?;
        Ok(count)
    }

    /// Get blocked contacts.
    pub async fn get_all_blocked(context: &Context) -> Result<Vec<ContactId>> {
        Contact::update_blocked_mailinglist_contacts(context)
            .await
            .context("cannot update blocked mailinglist contacts")?;

        let list = context
            .sql
            .query_map(
                "SELECT id FROM contacts WHERE id>? AND blocked!=0 ORDER BY last_seen DESC, id DESC;",
                (ContactId::LAST_SPECIAL,),
                |row| row.get::<_, ContactId>(0),
                |ids| {
                    ids.collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(Into::into)
                },
            )
            .await?;
        Ok(list)
    }

    /// Returns a textual summary of the encryption state for the contact.
    ///
    /// This function returns a string explaining the encryption state
    /// of the contact and if the connection is encrypted the
    /// fingerprints of the keys involved.
    pub async fn get_encrinfo(context: &Context, contact_id: ContactId) -> Result<String> {
        ensure!(
            !contact_id.is_special(),
            "Can not provide encryption info for special contact"
        );

        let mut ret = String::new();
        if let Ok(contact) = Contact::get_by_id(context, contact_id).await {
            let loginparam = LoginParam::load_configured_params(context).await?;
            let peerstate = Peerstate::from_addr(context, &contact.addr).await?;

            if let Some(peerstate) = peerstate.filter(|peerstate| {
                peerstate
                    .peek_key(PeerstateVerifiedStatus::Unverified)
                    .is_some()
            }) {
                let stock_message = match peerstate.prefer_encrypt {
                    EncryptPreference::Mutual => stock_str::e2e_preferred(context).await,
                    EncryptPreference::NoPreference => stock_str::e2e_available(context).await,
                    EncryptPreference::Reset => stock_str::encr_none(context).await,
                };

                let finger_prints = stock_str::finger_prints(context).await;
                ret += &format!("{stock_message}.\n{finger_prints}:");

                let fingerprint_self = load_self_public_key(context)
                    .await?
                    .fingerprint()
                    .to_string();
                let fingerprint_other_verified = peerstate
                    .peek_key(PeerstateVerifiedStatus::BidirectVerified)
                    .map(|k| k.fingerprint().to_string())
                    .unwrap_or_default();
                let fingerprint_other_unverified = peerstate
                    .peek_key(PeerstateVerifiedStatus::Unverified)
                    .map(|k| k.fingerprint().to_string())
                    .unwrap_or_default();
                if loginparam.addr < peerstate.addr {
                    cat_fingerprint(&mut ret, &loginparam.addr, &fingerprint_self, "");
                    cat_fingerprint(
                        &mut ret,
                        &peerstate.addr,
                        &fingerprint_other_verified,
                        &fingerprint_other_unverified,
                    );
                } else {
                    cat_fingerprint(
                        &mut ret,
                        &peerstate.addr,
                        &fingerprint_other_verified,
                        &fingerprint_other_unverified,
                    );
                    cat_fingerprint(&mut ret, &loginparam.addr, &fingerprint_self, "");
                }
            } else {
                ret += &stock_str::encr_none(context).await;
            }
        }

        Ok(ret)
    }

    /// Delete a contact so that it disappears from the corresponding lists.
    /// Depending on whether there are ongoing chats, deletion is done by physical deletion or hiding.
    /// The contact is deleted from the local device.
    ///
    /// May result in a `#DC_EVENT_CONTACTS_CHANGED` event.
    pub async fn delete(context: &Context, contact_id: ContactId) -> Result<()> {
        ensure!(!contact_id.is_special(), "Can not delete special contact");

        context
            .sql
            .transaction(move |transaction| {
                // make sure, the transaction starts with a write command and becomes EXCLUSIVE by that -
                // upgrading later may be impossible by races.
                let deleted_contacts = transaction.execute(
                    "DELETE FROM contacts WHERE id=?
                     AND (SELECT COUNT(*) FROM chats_contacts WHERE contact_id=?)=0;",
                    (contact_id, contact_id),
                )?;
                if deleted_contacts == 0 {
                    transaction.execute(
                        "UPDATE contacts SET origin=? WHERE id=?;",
                        (Origin::Hidden, contact_id),
                    )?;
                }
                Ok(())
            })
            .await?;

        context.emit_event(EventType::ContactsChanged(None));
        Ok(())
    }

    /// Updates `param` column in the database.
    pub async fn update_param(&self, context: &Context) -> Result<()> {
        context
            .sql
            .execute(
                "UPDATE contacts SET param=? WHERE id=?",
                (self.param.to_string(), self.id),
            )
            .await?;
        Ok(())
    }

    /// Updates `status` column in the database.
    pub async fn update_status(&self, context: &Context) -> Result<()> {
        context
            .sql
            .execute(
                "UPDATE contacts SET status=? WHERE id=?",
                (&self.status, self.id),
            )
            .await?;
        Ok(())
    }

    /// Get the ID of the contact.
    pub fn get_id(&self) -> ContactId {
        self.id
    }

    /// Get email address. The email address is always set for a contact.
    pub fn get_addr(&self) -> &str {
        &self.addr
    }

    /// Get name authorized by the contact.
    pub fn get_authname(&self) -> &str {
        &self.authname
    }

    /// Get the contact name. This is the name as modified by the local user.
    /// May be an empty string.
    ///
    /// This name is typically used in a form where the user can edit the name of a contact.
    /// To get a fine name to display in lists etc., use `Contact::get_display_name` or `Contact::get_name_n_addr`.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get display name. This is the name as defined by the contact himself,
    /// modified by the user or, if both are unset, the email address.
    ///
    /// This name is typically used in lists.
    /// To get the name editable in a formular, use `Contact::get_name`.
    pub fn get_display_name(&self) -> &str {
        if !self.name.is_empty() {
            return &self.name;
        }
        if !self.authname.is_empty() {
            return &self.authname;
        }
        &self.addr
    }

    /// Get a summary of authorized name and address.
    ///
    /// The returned string is either "Name (email@domain.com)" or just
    /// "email@domain.com" if the name is unset.
    ///
    /// This string is suitable for sending over email
    /// as it does not leak the locally set name.
    pub fn get_authname_n_addr(&self) -> String {
        if !self.authname.is_empty() {
            format!("{} ({})", self.authname, self.addr)
        } else {
            (&self.addr).into()
        }
    }

    /// Get a summary of name and address.
    ///
    /// The returned string is either "Name (email@domain.com)" or just
    /// "email@domain.com" if the name is unset.
    ///
    /// The result should only be used locally and never sent over the network
    /// as it leaks the local contact name.
    ///
    /// The summary is typically used when asking the user something about the contact.
    /// The attached email address makes the question unique, eg. "Chat with Alan Miller (am@uniquedomain.com)?"
    pub fn get_name_n_addr(&self) -> String {
        if !self.name.is_empty() {
            format!("{} ({})", self.name, self.addr)
        } else if !self.authname.is_empty() {
            format!("{} ({})", self.authname, self.addr)
        } else {
            (&self.addr).into()
        }
    }

    /// Get the contact's profile image.
    /// This is the image set by each remote user on their own
    /// using set_config(context, "selfavatar", image).
    pub async fn get_profile_image(&self, context: &Context) -> Result<Option<PathBuf>> {
        if self.id == ContactId::SELF {
            if let Some(p) = context.get_config(Config::Selfavatar).await? {
                return Ok(Some(PathBuf::from(p)));
            }
        } else if let Some(image_rel) = self.param.get(Param::ProfileImage) {
            if !image_rel.is_empty() {
                return Ok(Some(get_abs_path(context, Path::new(image_rel))));
            }
        }
        Ok(None)
    }

    /// Get a color for the contact.
    /// The color is calculated from the contact's email address
    /// and can be used for an fallback avatar with white initials
    /// as well as for headlines in bubbles of group chats.
    pub fn get_color(&self) -> u32 {
        str_to_color(&self.addr.to_lowercase())
    }

    /// Gets the contact's status.
    ///
    /// Status is the last signature received in a message from this contact.
    pub fn get_status(&self) -> &str {
        self.status.as_str()
    }

    /// Check if a contact was verified. E.g. by a secure-join QR code scan
    /// and if the key has not changed since this verification.
    ///
    /// The UI may draw a checkbox or something like that beside verified contacts.
    pub async fn is_verified(&self, context: &Context) -> Result<VerifiedStatus> {
        // We're always sort of secured-verified as we could verify the key on this device any time with the key
        // on this device
        if self.id == ContactId::SELF {
            return Ok(VerifiedStatus::BidirectVerified);
        }

        if let Some(peerstate) = Peerstate::from_addr(context, &self.addr).await? {
            if peerstate.is_using_verified_key() {
                return Ok(VerifiedStatus::BidirectVerified);
            }
        }

        Ok(VerifiedStatus::Unverified)
    }

    /// Returns the address that verified the contact.
    pub async fn get_verifier_addr(&self, context: &Context) -> Result<Option<String>> {
        Ok(Peerstate::from_addr(context, self.get_addr())
            .await?
            .and_then(|peerstate| peerstate.get_verifier().map(|addr| addr.to_owned())))
    }

    /// Returns the ContactId that verified the contact.
    pub async fn get_verifier_id(&self, context: &Context) -> Result<Option<ContactId>> {
        let Some(verifier_addr) = self.get_verifier_addr(context).await? else {
            return Ok(None);
        };

        if verifier_addr == self.addr {
            // Contact is directly verified via QR code.
            return Ok(Some(ContactId::SELF));
        }

        match Contact::lookup_id_by_addr(context, &verifier_addr, Origin::AddressBook).await? {
            Some(contact_id) => Ok(Some(contact_id)),
            None => {
                let addr = &self.addr;
                warn!(context, "Could not lookup contact with address {verifier_addr} which introduced {addr}.");
                Ok(None)
            }
        }
    }

    /// Returns the number of real (i.e. non-special) contacts in the database.
    pub async fn get_real_cnt(context: &Context) -> Result<usize> {
        if !context.sql.is_open().await {
            return Ok(0);
        }

        let count = context
            .sql
            .count(
                "SELECT COUNT(*) FROM contacts WHERE id>?;",
                (ContactId::LAST_SPECIAL,),
            )
            .await?;
        Ok(count)
    }

    /// Returns true if a contact with this ID exists.
    pub async fn real_exists_by_id(context: &Context, contact_id: ContactId) -> Result<bool> {
        if contact_id.is_special() {
            return Ok(false);
        }

        let exists = context
            .sql
            .exists("SELECT COUNT(*) FROM contacts WHERE id=?;", (contact_id,))
            .await?;
        Ok(exists)
    }

    /// Updates the origin of the contact, but only if new origin is higher than the current one.
    pub async fn scaleup_origin_by_id(
        context: &Context,
        contact_id: ContactId,
        origin: Origin,
    ) -> Result<()> {
        context
            .sql
            .execute(
                "UPDATE contacts SET origin=? WHERE id=? AND origin<?;",
                (origin, contact_id, origin),
            )
            .await?;
        Ok(())
    }
}

/// Returns false if addr is an invalid address, otherwise true.
pub fn may_be_valid_addr(addr: &str) -> bool {
    let res = EmailAddress::new(addr);
    res.is_ok()
}

/// Returns address with whitespace trimmed and `mailto:` prefix removed.
pub fn addr_normalize(addr: &str) -> &str {
    let norm = addr.trim();

    if norm.starts_with("mailto:") {
        norm.get(7..).unwrap_or(norm)
    } else {
        norm
    }
}

fn sanitize_name_and_addr(name: &str, addr: &str) -> (String, String) {
    static ADDR_WITH_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("(.*)<(.*)>").unwrap());
    if let Some(captures) = ADDR_WITH_NAME_REGEX.captures(addr.as_ref()) {
        (
            if name.is_empty() {
                strip_rtlo_characters(
                    &captures
                        .get(1)
                        .map_or("".to_string(), |m| normalize_name(m.as_str())),
                )
            } else {
                strip_rtlo_characters(name)
            },
            captures
                .get(2)
                .map_or("".to_string(), |m| m.as_str().to_string()),
        )
    } else {
        (strip_rtlo_characters(name), addr.to_string())
    }
}

async fn set_block_contact(
    context: &Context,
    contact_id: ContactId,
    new_blocking: bool,
) -> Result<()> {
    ensure!(
        !contact_id.is_special(),
        "Can't block special contact {}",
        contact_id
    );

    let contact = Contact::get_by_id(context, contact_id).await?;

    if contact.blocked != new_blocking {
        context
            .sql
            .execute(
                "UPDATE contacts SET blocked=? WHERE id=?;",
                (i32::from(new_blocking), contact_id),
            )
            .await?;

        // also (un)block all chats with _only_ this contact - we do not delete them to allow a
        // non-destructive blocking->unblocking.
        // (Maybe, beside normal chats (type=100) we should also block group chats with only this user.
        // However, I'm not sure about this point; it may be confusing if the user wants to add other people;
        // this would result in recreating the same group...)
        if context
            .sql
            .execute(
                r#"
UPDATE chats
SET blocked=?
WHERE type=? AND id IN (
  SELECT chat_id FROM chats_contacts WHERE contact_id=?
);
"#,
                (new_blocking, Chattype::Single, contact_id),
            )
            .await
            .is_ok()
        {
            Contact::mark_noticed(context, contact_id).await?;
            context.emit_event(EventType::ContactsChanged(Some(contact_id)));
        }

        // also unblock mailinglist
        // if the contact is a mailinglist address explicitly created to allow unblocking
        if !new_blocking && contact.origin == Origin::MailinglistAddress {
            if let Some((chat_id, _, _)) =
                chat::get_chat_id_by_grpid(context, &contact.addr).await?
            {
                chat_id.unblock(context).await?;
            }
        }
    }

    Ok(())
}

/// Set profile image for a contact.
///
/// The given profile image is expected to be already in the blob directory
/// as profile images can be set only by receiving messages, this should be always the case, however.
///
/// For contact SELF, the image is not saved in the contact-database but as Config::Selfavatar;
/// this typically happens if we see message with our own profile image, sent from another device.
pub(crate) async fn set_profile_image(
    context: &Context,
    contact_id: ContactId,
    profile_image: &AvatarAction,
    was_encrypted: bool,
) -> Result<()> {
    let mut contact = Contact::get_by_id(context, contact_id).await?;
    let changed = match profile_image {
        AvatarAction::Change(profile_image) => {
            if contact_id == ContactId::SELF {
                if was_encrypted {
                    context
                        .set_config(Config::Selfavatar, Some(profile_image))
                        .await?;
                } else {
                    info!(context, "Do not use unencrypted selfavatar.");
                }
            } else {
                contact.param.set(Param::ProfileImage, profile_image);
            }
            true
        }
        AvatarAction::Delete => {
            if contact_id == ContactId::SELF {
                if was_encrypted {
                    context.set_config(Config::Selfavatar, None).await?;
                } else {
                    info!(context, "Do not use unencrypted selfavatar deletion.");
                }
            } else {
                contact.param.remove(Param::ProfileImage);
            }
            true
        }
    };
    if changed {
        contact.update_param(context).await?;
        context.emit_event(EventType::ContactsChanged(Some(contact_id)));
    }
    Ok(())
}

/// Sets contact status.
///
/// For contact SELF, the status is not saved in the contact table, but as Config::Selfstatus.  This
/// is only done if message is sent from Delta Chat and it is encrypted, to synchronize signature
/// between Delta Chat devices.
pub(crate) async fn set_status(
    context: &Context,
    contact_id: ContactId,
    status: String,
    encrypted: bool,
    has_chat_version: bool,
) -> Result<()> {
    if contact_id == ContactId::SELF {
        if encrypted && has_chat_version {
            context
                .set_config(Config::Selfstatus, Some(&status))
                .await?;
        }
    } else {
        let mut contact = Contact::get_by_id(context, contact_id).await?;

        if contact.status != status {
            contact.status = status;
            contact.update_status(context).await?;
            context.emit_event(EventType::ContactsChanged(Some(contact_id)));
        }
    }
    Ok(())
}

/// Updates last seen timestamp of the contact if it is earlier than the given `timestamp`.
pub(crate) async fn update_last_seen(
    context: &Context,
    contact_id: ContactId,
    timestamp: i64,
) -> Result<()> {
    ensure!(
        !contact_id.is_special(),
        "Can not update special contact last seen timestamp"
    );

    if context
        .sql
        .execute(
            "UPDATE contacts SET last_seen = ?1 WHERE last_seen < ?1 AND id = ?2",
            (timestamp, contact_id),
        )
        .await?
        > 0
        && timestamp > time() - SEEN_RECENTLY_SECONDS
    {
        context
            .scheduler
            .interrupt_recently_seen(contact_id, timestamp)
            .await;
    }
    Ok(())
}

/// Normalize a name.
///
/// - Remove quotes (come from some bad MUA implementations)
/// - Trims the resulting string
///
/// Typically, this function is not needed as it is called implicitly by `Contact::add_address_book`.
pub fn normalize_name(full_name: &str) -> String {
    let full_name = full_name.trim();
    if full_name.is_empty() {
        return full_name.into();
    }

    match full_name.as_bytes() {
        [b'\'', .., b'\''] | [b'\"', .., b'\"'] | [b'<', .., b'>'] => full_name
            .get(1..full_name.len() - 1)
            .map_or("".to_string(), |s| s.trim().to_string()),
        _ => full_name.to_string(),
    }
}

fn cat_fingerprint(
    ret: &mut String,
    addr: &str,
    fingerprint_verified: &str,
    fingerprint_unverified: &str,
) {
    *ret += &format!(
        "\n\n{}:\n{}",
        addr,
        if !fingerprint_verified.is_empty() {
            fingerprint_verified
        } else {
            fingerprint_unverified
        },
    );
    if !fingerprint_verified.is_empty()
        && !fingerprint_unverified.is_empty()
        && fingerprint_verified != fingerprint_unverified
    {
        *ret += &format!("\n\n{addr} (alternative):\n{fingerprint_unverified}");
    }
}

/// Compares two email addresses, normalizing them beforehand.
pub fn addr_cmp(addr1: &str, addr2: &str) -> bool {
    let norm1 = addr_normalize(addr1).to_lowercase();
    let norm2 = addr_normalize(addr2).to_lowercase();

    norm1 == norm2
}

fn split_address_book(book: &str) -> Vec<(&str, &str)> {
    book.lines()
        .collect::<Vec<&str>>()
        .chunks(2)
        .filter_map(|chunk| {
            let name = chunk.first()?;
            let addr = chunk.get(1)?;
            Some((*name, *addr))
        })
        .collect()
}

#[derive(Debug)]
pub(crate) struct RecentlySeenInterrupt {
    contact_id: ContactId,
    timestamp: i64,
}

#[derive(Debug)]
pub(crate) struct RecentlySeenLoop {
    /// Task running "recently seen" loop.
    handle: task::JoinHandle<()>,

    interrupt_send: Sender<RecentlySeenInterrupt>,
}

impl RecentlySeenLoop {
    pub(crate) fn new(context: Context) -> Self {
        let (interrupt_send, interrupt_recv) = channel::bounded(1);

        let handle = task::spawn(Self::run(context, interrupt_recv));
        Self {
            handle,
            interrupt_send,
        }
    }

    async fn run(context: Context, interrupt: Receiver<RecentlySeenInterrupt>) {
        type MyHeapElem = (Reverse<i64>, ContactId);

        // Priority contains all recently seen sorted by the timestamp
        // when they become not recently seen.
        //
        // Initialize with contacts which are currently seen, but will
        // become unseen in the future.
        let mut unseen_queue: BinaryHeap<MyHeapElem> = context
            .sql
            .query_map(
                "SELECT id, last_seen FROM contacts
                 WHERE last_seen > ?",
                (time() - SEEN_RECENTLY_SECONDS,),
                |row| {
                    let contact_id: ContactId = row.get("id")?;
                    let last_seen: i64 = row.get("last_seen")?;
                    Ok((Reverse(last_seen + SEEN_RECENTLY_SECONDS), contact_id))
                },
                |rows| {
                    rows.collect::<std::result::Result<BinaryHeap<MyHeapElem>, _>>()
                        .map_err(Into::into)
                },
            )
            .await
            .unwrap_or_default();

        loop {
            let now = SystemTime::now();

            let (until, contact_id) =
                if let Some((Reverse(timestamp), contact_id)) = unseen_queue.peek() {
                    (
                        UNIX_EPOCH
                            + Duration::from_secs((*timestamp).try_into().unwrap_or(u64::MAX))
                            + Duration::from_secs(1),
                        Some(contact_id),
                    )
                } else {
                    // Sleep for 24 hours.
                    (now + Duration::from_secs(86400), None)
                };

            if let Ok(duration) = until.duration_since(now) {
                info!(
                    context,
                    "Recently seen loop waiting for {} or interrupt",
                    duration_to_str(duration)
                );

                match timeout(duration, interrupt.recv()).await {
                    Err(_) => {
                        // Timeout, notify about contact.
                        if let Some(contact_id) = contact_id {
                            context.emit_event(EventType::ContactsChanged(Some(*contact_id)));
                            unseen_queue.pop();
                        }
                    }
                    Ok(Err(err)) => {
                        warn!(
                            context,
                            "Error receiving an interruption in recently seen loop: {}", err
                        );
                        // Maybe the sender side is closed.
                        // Terminate the loop to avoid looping indefinitely.
                        return;
                    }
                    Ok(Ok(RecentlySeenInterrupt {
                        contact_id,
                        timestamp,
                    })) => {
                        // Received an interrupt.
                        unseen_queue.push((Reverse(timestamp + SEEN_RECENTLY_SECONDS), contact_id));
                    }
                }
            } else {
                info!(
                    context,
                    "Recently seen loop is not waiting, event is already due."
                );

                // Event is already in the past.
                if let Some(contact_id) = contact_id {
                    context.emit_event(EventType::ContactsChanged(Some(*contact_id)));
                }
                unseen_queue.pop();
            }
        }
    }

    pub(crate) fn interrupt(&self, contact_id: ContactId, timestamp: i64) {
        self.interrupt_send
            .try_send(RecentlySeenInterrupt {
                contact_id,
                timestamp,
            })
            .ok();
    }

    pub(crate) fn abort(self) {
        self.handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{get_chat_contacts, send_text_msg, Chat};
    use crate::chatlist::Chatlist;
    use crate::receive_imf::receive_imf;
    use crate::test_utils::{self, TestContext, TestContextManager};

    #[test]
    fn test_contact_id_values() {
        // Some FFI users need to have the values of these fixed, how naughty.  But let's
        // make sure we don't modify them anyway.
        assert_eq!(ContactId::UNDEFINED.to_u32(), 0);
        assert_eq!(ContactId::SELF.to_u32(), 1);
        assert_eq!(ContactId::INFO.to_u32(), 2);
        assert_eq!(ContactId::DEVICE.to_u32(), 5);
        assert_eq!(ContactId::LAST_SPECIAL.to_u32(), 9);
    }

    #[test]
    fn test_may_be_valid_addr() {
        assert_eq!(may_be_valid_addr(""), false);
        assert_eq!(may_be_valid_addr("user@domain.tld"), true);
        assert_eq!(may_be_valid_addr("uuu"), false);
        assert_eq!(may_be_valid_addr("dd.tt"), false);
        assert_eq!(may_be_valid_addr("tt.dd@uu"), true);
        assert_eq!(may_be_valid_addr("u@d"), true);
        assert_eq!(may_be_valid_addr("u@d."), false);
        assert_eq!(may_be_valid_addr("u@d.t"), true);
        assert_eq!(may_be_valid_addr("u@d.tt"), true);
        assert_eq!(may_be_valid_addr("u@.tt"), true);
        assert_eq!(may_be_valid_addr("@d.tt"), false);
        assert_eq!(may_be_valid_addr("<da@d.tt"), false);
        assert_eq!(may_be_valid_addr("sk <@d.tt>"), false);
        assert_eq!(may_be_valid_addr("as@sd.de>"), false);
        assert_eq!(may_be_valid_addr("ask dkl@dd.tt"), false);
        assert_eq!(may_be_valid_addr("user@domain.tld."), false);
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(&normalize_name(" hello world   "), "hello world");
        assert_eq!(&normalize_name("<"), "<");
        assert_eq!(&normalize_name(">"), ">");
        assert_eq!(&normalize_name("'"), "'");
        assert_eq!(&normalize_name("\""), "\"");
    }

    #[test]
    fn test_normalize_addr() {
        assert_eq!(addr_normalize("mailto:john@doe.com"), "john@doe.com");
        assert_eq!(addr_normalize("  hello@world.com   "), "hello@world.com");

        // normalisation preserves case to allow user-defined spelling.
        // however, case is ignored on addr_cmp()
        assert_ne!(addr_normalize("John@Doe.com"), "john@doe.com");
    }

    #[test]
    fn test_split_address_book() {
        let book = "Name one\nAddress one\nName two\nAddress two\nrest name";
        let list = split_address_book(book);
        assert_eq!(
            list,
            vec![("Name one", "Address one"), ("Name two", "Address two")]
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_contacts() -> Result<()> {
        let context = TestContext::new().await;

        assert!(context.get_all_self_addrs().await?.is_empty());

        // Bob is not in the contacts yet.
        let contacts = Contact::get_all(&context.ctx, 0, Some("bob")).await?;
        assert_eq!(contacts.len(), 0);

        let (id, _modified) = Contact::add_or_lookup(
            &context.ctx,
            "bob",
            ContactAddress::new("user@example.org")?,
            Origin::IncomingReplyTo,
        )
        .await?;
        assert_ne!(id, ContactId::UNDEFINED);

        let contact = Contact::get_by_id(&context.ctx, id).await.unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_authname(), "bob");
        assert_eq!(contact.get_display_name(), "bob");

        // Search by name.
        let contacts = Contact::get_all(&context.ctx, 0, Some("bob")).await?;
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts.get(0), Some(&id));

        // Search by address.
        let contacts = Contact::get_all(&context.ctx, 0, Some("user")).await?;
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts.get(0), Some(&id));

        let contacts = Contact::get_all(&context.ctx, 0, Some("alice")).await?;
        assert_eq!(contacts.len(), 0);

        // Set Bob name to "someone" manually.
        let (contact_bob_id, modified) = Contact::add_or_lookup(
            &context.ctx,
            "someone",
            ContactAddress::new("user@example.org")?,
            Origin::ManuallyCreated,
        )
        .await?;
        assert_eq!(contact_bob_id, id);
        assert_eq!(modified, Modifier::Modified);
        let contact = Contact::get_by_id(&context.ctx, id).await.unwrap();
        assert_eq!(contact.get_name(), "someone");
        assert_eq!(contact.get_authname(), "bob");
        assert_eq!(contact.get_display_name(), "someone");

        // Not searchable by authname, because it is not displayed.
        let contacts = Contact::get_all(&context.ctx, 0, Some("bob")).await?;
        assert_eq!(contacts.len(), 0);

        // Search by display name (same as manually set name).
        let contacts = Contact::get_all(&context.ctx, 0, Some("someone")).await?;
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts.get(0), Some(&id));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_is_self_addr() -> Result<()> {
        let t = TestContext::new().await;
        assert_eq!(t.is_self_addr("me@me.org").await?, false);

        t.configure_addr("you@you.net").await;
        assert_eq!(t.is_self_addr("me@me.org").await?, false);
        assert_eq!(t.is_self_addr("you@you.net").await?, true);

        Ok(())
    }

    #[test]
    fn test_contact_address() -> Result<()> {
        let alice_addr = "alice@example.org";
        let contact_address = ContactAddress::new(alice_addr)?;
        assert_eq!(contact_address.as_ref(), alice_addr);

        let invalid_addr = "<> foobar";
        assert!(ContactAddress::new(invalid_addr).is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_or_lookup() {
        // add some contacts, this also tests add_address_book()
        let t = TestContext::new().await;
        let book = concat!(
            "  Name one  \n one@eins.org \n",
            "Name two\ntwo@deux.net\n",
            "Invalid\n+1234567890\n", // invalid, should be ignored
            "\nthree@drei.sam\n",
            "Name two\ntwo@deux.net\n", // should not be added again
            "\nWonderland, Alice <alice@w.de>\n",
        );
        assert_eq!(Contact::add_address_book(&t, book).await.unwrap(), 4);

        // check first added contact, this modifies authname because it is empty
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t,
            "bla foo",
            ContactAddress::new("one@eins.org").unwrap(),
            Origin::IncomingUnknownTo,
        )
        .await
        .unwrap();
        assert!(!contact_id.is_special());
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_id(), contact_id);
        assert_eq!(contact.get_name(), "Name one");
        assert_eq!(contact.get_authname(), "bla foo");
        assert_eq!(contact.get_display_name(), "Name one");
        assert_eq!(contact.get_addr(), "one@eins.org");
        assert_eq!(contact.get_name_n_addr(), "Name one (one@eins.org)");

        // modify first added contact
        let (contact_id_test, sth_modified) = Contact::add_or_lookup(
            &t,
            "Real one",
            ContactAddress::new(" one@eins.org  ").unwrap(),
            Origin::ManuallyCreated,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "Real one");
        assert_eq!(contact.get_addr(), "one@eins.org");
        assert!(!contact.is_blocked());

        // check third added contact (contact without name)
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t,
            "",
            ContactAddress::new("three@drei.sam").unwrap(),
            Origin::IncomingUnknownTo,
        )
        .await
        .unwrap();
        assert!(!contact_id.is_special());
        assert_eq!(sth_modified, Modifier::None);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "three@drei.sam");
        assert_eq!(contact.get_addr(), "three@drei.sam");
        assert_eq!(contact.get_name_n_addr(), "three@drei.sam");

        // add name to third contact from incoming message (this becomes authorized name)
        let (contact_id_test, sth_modified) = Contact::add_or_lookup(
            &t,
            "m. serious",
            ContactAddress::new("three@drei.sam").unwrap(),
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name_n_addr(), "m. serious (three@drei.sam)");
        assert!(!contact.is_blocked());

        // manually edit name of third contact (does not changed authorized name)
        let (contact_id_test, sth_modified) = Contact::add_or_lookup(
            &t,
            "schnucki",
            ContactAddress::new("three@drei.sam").unwrap(),
            Origin::ManuallyCreated,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "m. serious");
        assert_eq!(contact.get_name_n_addr(), "schnucki (three@drei.sam)");
        assert!(!contact.is_blocked());

        // Fourth contact:
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t,
            "",
            ContactAddress::new("alice@w.de").unwrap(),
            Origin::IncomingUnknownTo,
        )
        .await
        .unwrap();
        assert!(!contact_id.is_special());
        assert_eq!(sth_modified, Modifier::None);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "Wonderland, Alice");
        assert_eq!(contact.get_display_name(), "Wonderland, Alice");
        assert_eq!(contact.get_addr(), "alice@w.de");
        assert_eq!(contact.get_name_n_addr(), "Wonderland, Alice (alice@w.de)");

        // check SELF
        let contact = Contact::get_by_id(&t, ContactId::SELF).await.unwrap();
        assert_eq!(contact.get_name(), stock_str::self_msg(&t).await);
        assert_eq!(contact.get_addr(), ""); // we're not configured
        assert!(!contact.is_blocked());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_contact_name_changes() -> Result<()> {
        let t = TestContext::new_alice().await;

        // first message creates contact and one-to-one-chat without name set
        receive_imf(
            &t,
            b"From: f@example.org\n\
                 To: alice@example.org\n\
                 Subject: foo\n\
                 Message-ID: <1234-1@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 29 May 2022 08:37:57 +0000\n\
                 \n\
                 hello one\n",
            false,
        )
        .await?;
        let chat_id = t.get_last_msg().await.get_chat_id();
        chat_id.accept(&t).await?;
        assert_eq!(Chat::load_from_db(&t, chat_id).await?.name, "f@example.org");
        let chatlist = Chatlist::try_load(&t, 0, Some("f@example.org"), None).await?;
        assert_eq!(chatlist.len(), 1);
        let contacts = get_chat_contacts(&t, chat_id).await?;
        let contact_id = contacts.first().unwrap();
        let contact = Contact::get_by_id(&t, *contact_id).await?;
        assert_eq!(contact.get_authname(), "");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "f@example.org");
        assert_eq!(contact.get_name_n_addr(), "f@example.org");
        let contacts = Contact::get_all(&t, 0, Some("f@example.org")).await?;
        assert_eq!(contacts.len(), 1);

        // second message inits the name
        receive_imf(
            &t,
            b"From: Flobbyfoo <f@example.org>\n\
                 To: alice@example.org\n\
                 Subject: foo\n\
                 Message-ID: <1234-2@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 29 May 2022 08:38:57 +0000\n\
                 \n\
                 hello two\n",
            false,
        )
        .await?;
        let chat_id = t.get_last_msg().await.get_chat_id();
        assert_eq!(Chat::load_from_db(&t, chat_id).await?.name, "Flobbyfoo");
        let chatlist = Chatlist::try_load(&t, 0, Some("flobbyfoo"), None).await?;
        assert_eq!(chatlist.len(), 1);
        let contact = Contact::get_by_id(&t, *contact_id).await?;
        assert_eq!(contact.get_authname(), "Flobbyfoo");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "Flobbyfoo");
        assert_eq!(contact.get_name_n_addr(), "Flobbyfoo (f@example.org)");
        let contacts = Contact::get_all(&t, 0, Some("f@example.org")).await?;
        assert_eq!(contacts.len(), 1);
        let contacts = Contact::get_all(&t, 0, Some("flobbyfoo")).await?;
        assert_eq!(contacts.len(), 1);

        // third message changes the name
        receive_imf(
            &t,
            b"From: Foo Flobby <f@example.org>\n\
                 To: alice@example.org\n\
                 Subject: foo\n\
                 Message-ID: <1234-3@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 29 May 2022 08:39:57 +0000\n\
                 \n\
                 hello three\n",
            false,
        )
        .await?;
        let chat_id = t.get_last_msg().await.get_chat_id();
        assert_eq!(Chat::load_from_db(&t, chat_id).await?.name, "Foo Flobby");
        let chatlist = Chatlist::try_load(&t, 0, Some("Flobbyfoo"), None).await?;
        assert_eq!(chatlist.len(), 0);
        let chatlist = Chatlist::try_load(&t, 0, Some("Foo Flobby"), None).await?;
        assert_eq!(chatlist.len(), 1);
        let contact = Contact::get_by_id(&t, *contact_id).await?;
        assert_eq!(contact.get_authname(), "Foo Flobby");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "Foo Flobby");
        assert_eq!(contact.get_name_n_addr(), "Foo Flobby (f@example.org)");
        let contacts = Contact::get_all(&t, 0, Some("f@example.org")).await?;
        assert_eq!(contacts.len(), 1);
        let contacts = Contact::get_all(&t, 0, Some("flobbyfoo")).await?;
        assert_eq!(contacts.len(), 0);
        let contacts = Contact::get_all(&t, 0, Some("Foo Flobby")).await?;
        assert_eq!(contacts.len(), 1);

        // change name manually
        let test_id = Contact::create(&t, "Falk", "f@example.org").await?;
        assert_eq!(*contact_id, test_id);
        assert_eq!(Chat::load_from_db(&t, chat_id).await?.name, "Falk");
        let chatlist = Chatlist::try_load(&t, 0, Some("Falk"), None).await?;
        assert_eq!(chatlist.len(), 1);
        let contact = Contact::get_by_id(&t, *contact_id).await?;
        assert_eq!(contact.get_authname(), "Foo Flobby");
        assert_eq!(contact.get_name(), "Falk");
        assert_eq!(contact.get_display_name(), "Falk");
        assert_eq!(contact.get_name_n_addr(), "Falk (f@example.org)");
        let contacts = Contact::get_all(&t, 0, Some("f@example.org")).await?;
        assert_eq!(contacts.len(), 1);
        let contacts = Contact::get_all(&t, 0, Some("falk")).await?;
        assert_eq!(contacts.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_delete() -> Result<()> {
        let alice = TestContext::new_alice().await;

        assert!(Contact::delete(&alice, ContactId::SELF).await.is_err());

        // Create Bob contact
        let (contact_id, _) = Contact::add_or_lookup(
            &alice,
            "Bob",
            ContactAddress::new("bob@example.net")?,
            Origin::ManuallyCreated,
        )
        .await?;
        let chat = alice
            .create_chat_with_contact("Bob", "bob@example.net")
            .await;
        assert_eq!(
            Contact::get_all(&alice, 0, Some("bob@example.net"))
                .await?
                .len(),
            1
        );

        // If a contact has ongoing chats, contact is only hidden on deletion
        Contact::delete(&alice, contact_id).await?;
        let contact = Contact::get_by_id(&alice, contact_id).await?;
        assert_eq!(contact.origin, Origin::Hidden);
        assert_eq!(
            Contact::get_all(&alice, 0, Some("bob@example.net"))
                .await?
                .len(),
            0
        );

        // Delete chat.
        chat.get_id().delete(&alice).await?;

        // Can delete contact physically now
        Contact::delete(&alice, contact_id).await?;
        assert!(Contact::get_by_id(&alice, contact_id).await.is_err());
        assert_eq!(
            Contact::get_all(&alice, 0, Some("bob@example.net"))
                .await?
                .len(),
            0
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_delete_and_recreate_contact() -> Result<()> {
        let t = TestContext::new_alice().await;

        // test recreation after physical deletion
        let contact_id1 = Contact::create(&t, "Foo", "foo@bar.de").await?;
        assert_eq!(Contact::get_all(&t, 0, Some("foo@bar.de")).await?.len(), 1);
        Contact::delete(&t, contact_id1).await?;
        assert!(Contact::get_by_id(&t, contact_id1).await.is_err());
        assert_eq!(Contact::get_all(&t, 0, Some("foo@bar.de")).await?.len(), 0);
        let contact_id2 = Contact::create(&t, "Foo", "foo@bar.de").await?;
        assert_ne!(contact_id2, contact_id1);
        assert_eq!(Contact::get_all(&t, 0, Some("foo@bar.de")).await?.len(), 1);

        // test recreation after hiding
        t.create_chat_with_contact("Foo", "foo@bar.de").await;
        Contact::delete(&t, contact_id2).await?;
        let contact = Contact::get_by_id(&t, contact_id2).await?;
        assert_eq!(contact.origin, Origin::Hidden);
        assert_eq!(Contact::get_all(&t, 0, Some("foo@bar.de")).await?.len(), 0);

        let contact_id3 = Contact::create(&t, "Foo", "foo@bar.de").await?;
        let contact = Contact::get_by_id(&t, contact_id3).await?;
        assert_eq!(contact.origin, Origin::ManuallyCreated);
        assert_eq!(contact_id3, contact_id2);
        assert_eq!(Contact::get_all(&t, 0, Some("foo@bar.de")).await?.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_remote_authnames() {
        let t = TestContext::new().await;

        // incoming mail `From: bob1 <bob@example.org>` - this should init authname
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t,
            "bob1",
            ContactAddress::new("bob@example.org").unwrap(),
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert!(!contact_id.is_special());
        assert_eq!(sth_modified, Modifier::Created);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob1");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "bob1");

        // incoming mail `From: bob2 <bob@example.org>` - this should update authname
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t,
            "bob2",
            ContactAddress::new("bob@example.org").unwrap(),
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert!(!contact_id.is_special());
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob2");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "bob2");

        // manually edit name to "bob3" - authname should be still be "bob2" as given in `From:` above
        let contact_id = Contact::create(&t, "bob3", "bob@example.org")
            .await
            .unwrap();
        assert!(!contact_id.is_special());
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob2");
        assert_eq!(contact.get_name(), "bob3");
        assert_eq!(contact.get_display_name(), "bob3");

        // incoming mail `From: bob4 <bob@example.org>` - this should update authname, manually given name is still "bob3"
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t,
            "bob4",
            ContactAddress::new("bob@example.org").unwrap(),
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert!(!contact_id.is_special());
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob4");
        assert_eq!(contact.get_name(), "bob3");
        assert_eq!(contact.get_display_name(), "bob3");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_remote_authnames_create_empty() {
        let t = TestContext::new().await;

        // manually create "claire@example.org" without a given name
        let contact_id = Contact::create(&t, "", "claire@example.org").await.unwrap();
        assert!(!contact_id.is_special());
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "claire@example.org");

        // incoming mail `From: claire1 <claire@example.org>` - this should update authname
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t,
            "claire1",
            ContactAddress::new("claire@example.org").unwrap(),
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "claire1");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "claire1");

        // incoming mail `From: claire2 <claire@example.org>` - this should update authname
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t,
            "claire2",
            ContactAddress::new("claire@example.org").unwrap(),
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "claire2");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "claire2");
    }

    /// Regression test.
    ///
    /// In the past, "Not Bob" name was stuck until "Bob" changed the name to "Not Bob" and back in
    /// the "From:" field or user set the name to empty string manually.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_remote_authnames_update_to() -> Result<()> {
        let t = TestContext::new().await;

        // Incoming message from Bob.
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t,
            "Bob",
            ContactAddress::new("bob@example.org")?,
            Origin::IncomingUnknownFrom,
        )
        .await?;
        assert_eq!(sth_modified, Modifier::Created);
        let contact = Contact::get_by_id(&t, contact_id).await?;
        assert_eq!(contact.get_display_name(), "Bob");

        // Incoming message from someone else with "Not Bob" <bob@example.org> in the "To:" field.
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t,
            "Not Bob",
            ContactAddress::new("bob@example.org")?,
            Origin::IncomingUnknownTo,
        )
        .await?;
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::get_by_id(&t, contact_id).await?;
        assert_eq!(contact.get_display_name(), "Not Bob");

        // Incoming message from Bob, changing the name back.
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t,
            "Bob",
            ContactAddress::new("bob@example.org")?,
            Origin::IncomingUnknownFrom,
        )
        .await?;
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified); // This was None until the bugfix
        let contact = Contact::get_by_id(&t, contact_id).await?;
        assert_eq!(contact.get_display_name(), "Bob");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_remote_authnames_edit_empty() {
        let t = TestContext::new().await;

        // manually create "dave@example.org"
        let contact_id = Contact::create(&t, "dave1", "dave@example.org")
            .await
            .unwrap();
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "");
        assert_eq!(contact.get_name(), "dave1");
        assert_eq!(contact.get_display_name(), "dave1");

        // incoming mail `From: dave2 <dave@example.org>` - this should update authname
        Contact::add_or_lookup(
            &t,
            "dave2",
            ContactAddress::new("dave@example.org").unwrap(),
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "dave2");
        assert_eq!(contact.get_name(), "dave1");
        assert_eq!(contact.get_display_name(), "dave1");

        // manually clear the name
        Contact::create(&t, "", "dave@example.org").await.unwrap();
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "dave2");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "dave2");
    }

    #[test]
    fn test_addr_cmp() {
        assert!(addr_cmp("AA@AA.ORG", "aa@aa.ORG"));
        assert!(addr_cmp(" aa@aa.ORG ", "AA@AA.ORG"));
        assert!(addr_cmp(" mailto:AA@AA.ORG", "Aa@Aa.orG"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_name_in_address() {
        let t = TestContext::new().await;

        let contact_id = Contact::create(&t, "", "<dave@example.org>").await.unwrap();
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_addr(), "dave@example.org");

        let contact_id = Contact::create(&t, "", "Mueller, Dave <dave@example.org>")
            .await
            .unwrap();
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "Mueller, Dave");
        assert_eq!(contact.get_addr(), "dave@example.org");

        let contact_id = Contact::create(&t, "name1", "name2 <dave@example.org>")
            .await
            .unwrap();
        let contact = Contact::get_by_id(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "name1");
        assert_eq!(contact.get_addr(), "dave@example.org");

        assert!(Contact::create(&t, "", "<dskjfdslk@sadklj.dk")
            .await
            .is_err());
        assert!(Contact::create(&t, "", "<dskjf>dslk@sadklj.dk>")
            .await
            .is_err());
        assert!(Contact::create(&t, "", "dskjfdslksadklj.dk").await.is_err());
        assert!(Contact::create(&t, "", "dskjfdslk@sadklj.dk>")
            .await
            .is_err());
        assert!(Contact::create(&t, "", "dskjf dslk@d.e").await.is_err());
        assert!(Contact::create(&t, "", "<dskjf dslk@sadklj.dk")
            .await
            .is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lookup_id_by_addr() {
        let t = TestContext::new().await;

        let id = Contact::lookup_id_by_addr(&t.ctx, "the.other@example.net", Origin::Unknown)
            .await
            .unwrap();
        assert!(id.is_none());

        let other_id = Contact::create(&t.ctx, "The Other", "the.other@example.net")
            .await
            .unwrap();
        let id = Contact::lookup_id_by_addr(&t.ctx, "the.other@example.net", Origin::Unknown)
            .await
            .unwrap();
        assert_eq!(id, Some(other_id));

        let alice = TestContext::new_alice().await;

        let id = Contact::lookup_id_by_addr(&alice.ctx, "alice@example.org", Origin::Unknown)
            .await
            .unwrap();
        assert_eq!(id, Some(ContactId::SELF));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_contact_get_color() -> Result<()> {
        let t = TestContext::new().await;
        let contact_id = Contact::create(&t, "name", "name@example.net").await?;
        let color1 = Contact::get_by_id(&t, contact_id).await?.get_color();
        assert_eq!(color1, 0xA739FF);

        let t = TestContext::new().await;
        let contact_id = Contact::create(&t, "prename name", "name@example.net").await?;
        let color2 = Contact::get_by_id(&t, contact_id).await?.get_color();
        assert_eq!(color2, color1);

        let t = TestContext::new().await;
        let contact_id = Contact::create(&t, "Name", "nAme@exAmple.NET").await?;
        let color3 = Contact::get_by_id(&t, contact_id).await?.get_color();
        assert_eq!(color3, color1);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_contact_get_encrinfo() -> Result<()> {
        let alice = TestContext::new_alice().await;

        // Return error for special IDs
        let encrinfo = Contact::get_encrinfo(&alice, ContactId::SELF).await;
        assert!(encrinfo.is_err());
        let encrinfo = Contact::get_encrinfo(&alice, ContactId::DEVICE).await;
        assert!(encrinfo.is_err());

        let (contact_bob_id, _modified) = Contact::add_or_lookup(
            &alice,
            "Bob",
            ContactAddress::new("bob@example.net")?,
            Origin::ManuallyCreated,
        )
        .await?;

        let encrinfo = Contact::get_encrinfo(&alice, contact_bob_id).await?;
        assert_eq!(encrinfo, "No encryption");

        let bob = TestContext::new_bob().await;
        let chat_alice = bob
            .create_chat_with_contact("Alice", "alice@example.org")
            .await;
        send_text_msg(&bob, chat_alice.id, "Hello".to_string()).await?;
        let msg = bob.pop_sent_msg().await;
        alice.recv_msg(&msg).await;

        let encrinfo = Contact::get_encrinfo(&alice, contact_bob_id).await?;
        assert_eq!(
            encrinfo,
            "End-to-end encryption preferred.
Fingerprints:

alice@example.org:
2E6F A2CB 23B5 32D7 2863
4B58 64B0 8F61 A9ED 9443

bob@example.net:
CCCB 5AA9 F6E1 141C 9431
65F1 DB18 B18C BCF7 0487"
        );
        Ok(())
    }

    /// Tests that status is synchronized when sending encrypted BCC-self messages and not
    /// synchronized when the message is not encrypted.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_synchronize_status() -> Result<()> {
        // Alice has two devices.
        let alice1 = TestContext::new_alice().await;
        let alice2 = TestContext::new_alice().await;

        // Bob has one device.
        let bob = TestContext::new_bob().await;

        let default_status = alice1.get_config(Config::Selfstatus).await?;

        alice1
            .set_config(Config::Selfstatus, Some("New status"))
            .await?;
        let chat = alice1
            .create_chat_with_contact("Bob", "bob@example.net")
            .await;

        // Alice sends a message to Bob from the first device.
        send_text_msg(&alice1, chat.id, "Hello".to_string()).await?;
        let sent_msg = alice1.pop_sent_msg().await;

        // Message is not encrypted.
        let message = sent_msg.load_from_db().await;
        assert!(!message.get_showpadlock());

        // Alice's second devices receives a copy of outgoing message.
        alice2.recv_msg(&sent_msg).await;

        // Bob receives message.
        bob.recv_msg(&sent_msg).await;

        // Message was not encrypted, so status is not copied.
        assert_eq!(alice2.get_config(Config::Selfstatus).await?, default_status);

        // Bob replies.
        let chat = bob
            .create_chat_with_contact("Alice", "alice@example.org")
            .await;

        send_text_msg(&bob, chat.id, "Reply".to_string()).await?;
        let sent_msg = bob.pop_sent_msg().await;
        alice1.recv_msg(&sent_msg).await;
        alice2.recv_msg(&sent_msg).await;

        // Alice sends second message.
        send_text_msg(&alice1, chat.id, "Hello".to_string()).await?;
        let sent_msg = alice1.pop_sent_msg().await;

        // Second message is encrypted.
        let message = sent_msg.load_from_db().await;
        assert!(message.get_showpadlock());

        // Alice's second devices receives a copy of second outgoing message.
        alice2.recv_msg(&sent_msg).await;

        assert_eq!(
            alice2.get_config(Config::Selfstatus).await?,
            Some("New status".to_string())
        );

        Ok(())
    }

    /// Tests that DC_EVENT_SELFAVATAR_CHANGED is emitted on avatar changes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_selfavatar_changed_event() -> Result<()> {
        // Alice has two devices.
        let alice1 = TestContext::new_alice().await;
        let alice2 = TestContext::new_alice().await;

        // Bob has one device.
        let bob = TestContext::new_bob().await;

        assert_eq!(alice1.get_config(Config::Selfavatar).await?, None);

        let avatar_src = alice1.get_blobdir().join("avatar.png");
        tokio::fs::write(&avatar_src, test_utils::AVATAR_900x900_BYTES).await?;

        alice1
            .set_config(Config::Selfavatar, Some(avatar_src.to_str().unwrap()))
            .await?;

        alice1
            .evtracker
            .get_matching(|e| matches!(e, EventType::SelfavatarChanged))
            .await;

        // Bob sends a message so that Alice can encrypt to him.
        let chat = bob
            .create_chat_with_contact("Alice", "alice@example.org")
            .await;

        send_text_msg(&bob, chat.id, "Reply".to_string()).await?;
        let sent_msg = bob.pop_sent_msg().await;
        alice1.recv_msg(&sent_msg).await;
        alice2.recv_msg(&sent_msg).await;

        // Alice sends a message.
        let alice1_chat_id = alice1.get_last_msg().await.chat_id;
        alice1_chat_id.accept(&alice1).await?;
        send_text_msg(&alice1, alice1_chat_id, "Hello".to_string()).await?;
        let sent_msg = alice1.pop_sent_msg().await;

        // The message is encrypted.
        let message = sent_msg.load_from_db().await;
        assert!(message.get_showpadlock());

        // Alice's second device receives a copy of the outgoing message.
        alice2.recv_msg(&sent_msg).await;

        // Alice's second device applies the selfavatar.
        assert!(alice2.get_config(Config::Selfavatar).await?.is_some());
        alice2
            .evtracker
            .get_matching(|e| matches!(e, EventType::SelfavatarChanged))
            .await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_last_seen() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let (contact_id, _) = Contact::add_or_lookup(
            &alice,
            "Bob",
            ContactAddress::new("bob@example.net")?,
            Origin::ManuallyCreated,
        )
        .await?;
        let contact = Contact::get_by_id(&alice, contact_id).await?;
        assert_eq!(contact.last_seen(), 0);

        let mime = br#"Subject: Hello
Message-ID: message@example.net
To: Alice <alice@example.org>
From: Bob <bob@example.net>
Content-Type: text/plain; charset=utf-8; format=flowed; delsp=no
Chat-Version: 1.0
Date: Sun, 22 Mar 2020 22:37:55 +0000

Hi."#;
        receive_imf(&alice, mime, false).await?;
        let msg = alice.get_last_msg().await;

        let timestamp = msg.get_timestamp();
        assert!(timestamp > 0);
        let contact = Contact::get_by_id(&alice, contact_id).await?;
        assert_eq!(contact.last_seen(), timestamp);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_was_seen_recently() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        let chat = alice.create_chat(&bob).await;
        let sent_msg = alice.send_text(chat.id, "moin").await;

        let chat = bob.create_chat(&alice).await;
        let contacts = chat::get_chat_contacts(&bob, chat.id).await?;
        let contact = Contact::get_by_id(&bob, *contacts.first().unwrap()).await?;
        assert!(!contact.was_seen_recently());

        bob.recv_msg(&sent_msg).await;
        let contact = Contact::get_by_id(&bob, *contacts.first().unwrap()).await?;
        assert!(contact.was_seen_recently());

        let self_contact = Contact::get_by_id(&bob, ContactId::SELF).await?;
        assert!(!self_contact.was_seen_recently());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_verified_by_none() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        let contact_id = Contact::create(&alice, "Bob", "bob@example.net").await?;
        let contact = Contact::get_by_id(&alice, contact_id).await?;
        assert!(contact.get_verifier_addr(&alice).await?.is_none());
        assert!(contact.get_verifier_id(&alice).await?.is_none());

        // Receive a message from Bob to create a peerstate.
        let chat = bob.create_chat(&alice).await;
        let sent_msg = bob.send_text(chat.id, "moin").await;
        alice.recv_msg(&sent_msg).await;

        let contact = Contact::get_by_id(&alice, contact_id).await?;
        assert!(contact.get_verifier_addr(&alice).await?.is_none());
        assert!(contact.get_verifier_id(&alice).await?.is_none());

        Ok(())
    }
}
