//! Contacts module

use anyhow::{bail, ensure, format_err, Context as _, Result};
use async_std::path::PathBuf;
use deltachat_derive::{FromSql, ToSql};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::aheader::EncryptPreference;
use crate::chat::ChatId;
use crate::color::str_to_color;
use crate::config::Config;
use crate::constants::{
    Chattype, DC_CHAT_ID_DEADDROP, DC_CONTACT_ID_DEVICE, DC_CONTACT_ID_DEVICE_ADDR,
    DC_CONTACT_ID_LAST_SPECIAL, DC_CONTACT_ID_SELF, DC_GCL_ADD_SELF, DC_GCL_VERIFIED_ONLY,
};
use crate::context::Context;
use crate::dc_tools::{dc_get_abs_path, improve_single_line_input, EmailAddress};
use crate::events::EventType;
use crate::key::{DcKey, SignedPublicKey};
use crate::login_param::LoginParam;
use crate::message::MessageState;
use crate::mimeparser::AvatarAction;
use crate::param::{Param, Params};
use crate::peerstate::{Peerstate, PeerstateVerifiedStatus};
use crate::stock_str;

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
    ///
    /// Special message IDs:
    /// - DC_CONTACT_ID_SELF (1) - this is the owner of the account with the email-address set by
    ///   `dc_set_config` using "addr".
    ///
    /// Normal contact IDs are larger than these special ones (larger than DC_CONTACT_ID_LAST_SPECIAL).
    pub id: u32,

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

    /// Blocked state. Use dc_contact_is_blocked to access this field.
    pub blocked: bool,

    /// The origin/source of the contact.
    pub origin: Origin,

    /// Parameters as Param::ProfileImage
    pub param: Params,

    /// Last seen message signature for this contact, to be displayed in the profile.
    status: String,
}

/// Possible origins of a contact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromPrimitive, ToPrimitive, FromSql, ToSql,
)]
#[repr(i32)]
pub enum Origin {
    Unknown = 0,

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

    /// set on Alice's side for contacts like Bob that have scanned the QR code offered by her. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling dc_contact_is_verified() !
    SecurejoinInvited = 0x0100_0000,

    /// set on Bob's side for contacts scanned and verified from a QR code. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling dc_contact_is_verified() !
    SecurejoinJoined = 0x0200_0000,

    /// contact added mannually by dc_create_contact(), this should be the largest origin as otherwise the user cannot modify the names
    ManuallyCreated = 0x0400_0000,
}

impl Default for Origin {
    fn default() -> Self {
        Origin::Unknown
    }
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
pub enum VerifiedStatus {
    /// Contact is not verified.
    Unverified = 0,
    // TODO: is this a thing?
    Verified = 1,
    /// SELF and contact have verified their fingerprints in both directions; in the UI typically checkmarks are shown.
    BidirectVerified = 2,
}

impl Contact {
    pub async fn load_from_db(context: &Context, contact_id: u32) -> crate::sql::Result<Self> {
        let mut res = context
            .sql
            .query_row(
                "SELECT c.name, c.addr, c.origin, c.blocked, c.authname, c.param, c.status
               FROM contacts c
              WHERE c.id=?;",
                paramsv![contact_id as i32],
                |row| {
                    let contact = Self {
                        id: contact_id,
                        name: row.get::<_, String>(0)?,
                        authname: row.get::<_, String>(4)?,
                        addr: row.get::<_, String>(1)?,
                        blocked: row.get::<_, Option<i32>>(3)?.unwrap_or_default() != 0,
                        origin: row.get(2)?,
                        param: row.get::<_, String>(5)?.parse().unwrap_or_default(),
                        status: row.get(6).unwrap_or_default(),
                    };
                    Ok(contact)
                },
            )
            .await?;
        if contact_id == DC_CONTACT_ID_SELF {
            res.name = stock_str::self_msg(context).await;
            res.addr = context
                .get_config(Config::ConfiguredAddr)
                .await
                .unwrap_or_default();
            res.status = context
                .get_config(Config::Selfstatus)
                .await
                .unwrap_or_default();
        } else if contact_id == DC_CONTACT_ID_DEVICE {
            res.name = stock_str::device_messages(context).await;
            res.addr = DC_CONTACT_ID_DEVICE_ADDR.to_string();
        }
        Ok(res)
    }

    /// Returns `true` if this contact is blocked.
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Check if a contact is blocked.
    pub async fn is_blocked_load(context: &Context, id: u32) -> bool {
        Self::load_from_db(context, id)
            .await
            .map(|contact| contact.blocked)
            .unwrap_or_default()
    }

    /// Block the given contact.
    pub async fn block(context: &Context, id: u32) {
        set_block_contact(context, id, true).await;
    }

    /// Unblock the given contact.
    pub async fn unblock(context: &Context, id: u32) {
        set_block_contact(context, id, false).await;
    }

    /// Add a single contact as a result of an _explicit_ user action.
    ///
    /// We assume, the contact name, if any, is entered by the user and is used "as is" therefore,
    /// normalize() is *not* called for the name. If the contact is blocked, it is unblocked.
    ///
    /// To add a number of contacts, see `dc_add_address_book()` which is much faster for adding
    /// a bunch of addresses.
    ///
    /// May result in a `#DC_EVENT_CONTACTS_CHANGED` event.
    pub async fn create(
        context: &Context,
        name: impl AsRef<str>,
        addr: impl AsRef<str>,
    ) -> Result<u32> {
        let name = improve_single_line_input(name);
        ensure!(
            !addr.as_ref().is_empty(),
            "Cannot create contact with empty address"
        );

        let (name, addr) = sanitize_name_and_addr(name, addr);

        let (contact_id, sth_modified) =
            Contact::add_or_lookup(context, name, addr, Origin::ManuallyCreated).await?;
        let blocked = Contact::is_blocked_load(context, contact_id).await;
        match sth_modified {
            Modifier::None => {}
            Modifier::Modified | Modifier::Created => {
                context.emit_event(EventType::ContactsChanged(Some(contact_id)))
            }
        }
        if blocked {
            Contact::unblock(context, contact_id).await;
        }

        Ok(contact_id)
    }

    /// Mark messages from a contact as noticed.
    /// The contact is expected to belong to the deaddrop,
    /// therefore, DC_EVENT_MSGS_NOTICED(DC_CHAT_ID_DEADDROP) is emitted.
    pub async fn mark_noticed(context: &Context, id: u32) {
        if context
            .sql
            .execute(
                "UPDATE msgs SET state=? WHERE from_id=? AND state=?;",
                paramsv![MessageState::InNoticed, id as i32, MessageState::InFresh],
            )
            .await
            .is_ok()
        {
            context.emit_event(EventType::MsgsNoticed(ChatId::new(DC_CHAT_ID_DEADDROP)));
        }
    }

    /// Check if an e-mail address belongs to a known and unblocked contact.
    ///
    /// Known and unblocked contacts will be returned by `dc_get_contacts()`.
    ///
    /// To validate an e-mail address independently of the contact database
    /// use `dc_may_be_valid_addr()`.
    pub async fn lookup_id_by_addr(
        context: &Context,
        addr: impl AsRef<str>,
        min_origin: Origin,
    ) -> Result<Option<u32>> {
        if addr.as_ref().is_empty() {
            bail!("lookup_id_by_addr: empty address");
        }

        let addr_normalized = addr_normalize(addr.as_ref());

        if let Some(addr_self) = context.get_config(Config::ConfiguredAddr).await {
            if addr_cmp(addr_normalized, addr_self) {
                return Ok(Some(DC_CONTACT_ID_SELF));
            }
        }
        context.sql.query_get_value_result(
            "SELECT id FROM contacts WHERE addr=?1 COLLATE NOCASE AND id>?2 AND origin>=?3 AND blocked=0;",
            paramsv![
                addr_normalized,
                DC_CONTACT_ID_LAST_SPECIAL as i32,
                min_origin as u32,
            ],
        )
            .await
            .context("lookup_id_by_addr: SQL query failed")
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
    /// Returns the contact_id and a `Modifier` value indicating if a modification occured.
    pub(crate) async fn add_or_lookup(
        context: &Context,
        name: impl AsRef<str>,
        addr: impl AsRef<str>,
        mut origin: Origin,
    ) -> Result<(u32, Modifier)> {
        let mut sth_modified = Modifier::None;

        ensure!(
            !addr.as_ref().is_empty(),
            "Can not add_or_lookup empty address"
        );
        ensure!(origin != Origin::Unknown, "Missing valid origin");

        let addr = addr_normalize(addr.as_ref()).to_string();
        let addr_self = context
            .get_config(Config::ConfiguredAddr)
            .await
            .unwrap_or_default();

        if addr_cmp(&addr, addr_self) {
            return Ok((DC_CONTACT_ID_SELF, sth_modified));
        }

        if !may_be_valid_addr(&addr) {
            warn!(
                context,
                "Bad address \"{}\" for contact \"{}\".",
                addr,
                if !name.as_ref().is_empty() {
                    name.as_ref()
                } else {
                    "<unset>"
                },
            );
            bail!("Bad address supplied: {:?}", addr);
        }

        let mut name = name.as_ref();
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
                name = "";
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
        let mut row_id = 0;

        if let Ok((id, row_name, row_addr, row_origin, row_authname)) = context.sql.query_row(
            "SELECT id, name, addr, origin, authname FROM contacts WHERE addr=? COLLATE NOCASE;",
            paramsv![addr.to_string()],
            |row| {
                let row_id = row.get(0)?;
                let row_name: String = row.get(1)?;
                let row_addr: String = row.get(2)?;
                let row_origin: Origin = row.get(3)?;
                let row_authname: String = row.get(4)?;

                Ok((row_id, row_name, row_addr, row_origin, row_authname))
            },
        )
        .await {
            let update_name = manual && name != row_name;
            let update_authname =
                !manual && name != row_authname && !name.is_empty() &&
                (origin >= row_origin || origin == Origin::IncomingUnknownFrom || row_authname.is_empty());

            row_id = id;
            if origin as i32 >= row_origin as i32 && addr != row_addr {
                update_addr = true;
            }
            if update_name || update_authname || update_addr || origin > row_origin {
                let new_name = if update_name {
                    name.to_string()
                } else {
                    row_name
                };

                context
                    .sql
                    .execute(
                        "UPDATE contacts SET name=?, addr=?, origin=?, authname=? WHERE id=?;",
                        paramsv![
                            new_name,
                            if update_addr { addr.to_string() } else { row_addr },
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
                        ],
                    )
                    .await
                    .ok();

                if update_name {
                    // Update the contact name also if it is used as a group name.
                    // This is one of the few duplicated data, however, getting the chat list is easier this way.
                    let chat_id = context.sql.query_get_value::<i32>(
                        context,
                        "SELECT id FROM chats WHERE type=? AND id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?)",
                        paramsv![Chattype::Single, row_id]
                    ).await;
                    if let Some(chat_id) = chat_id {
                        match context.sql.execute("UPDATE chats SET name=? WHERE id=? AND name!=?1", paramsv![new_name, chat_id]).await {
                            Err(err) => warn!(context, "Can't update chat name: {}", err),
                            Ok(count) => if count > 0 {
                                // Chat name updated
                                context.emit_event(EventType::ChatModified(ChatId::new(chat_id as u32)));
                            }
                        }
                    }
                }
                sth_modified = Modifier::Modified;
            }
        } else {
            let update_name = manual;
            let update_authname = !manual;

            if context
                .sql
                .execute(
                    "INSERT INTO contacts (name, addr, origin, authname) VALUES(?, ?, ?, ?);",
                    paramsv![
                        if update_name { name.to_string() } else { "".to_string() },
                        addr,
                        origin,
                        if update_authname { name.to_string() } else { "".to_string() }
                    ],
                )
                .await
                .is_ok()
            {
                row_id = context
                    .sql
                    .get_rowid(context, "contacts", "addr", &addr)
                    .await?;
                sth_modified = Modifier::Created;
                info!(context, "added contact id={} addr={}", row_id, &addr);
            } else {
                error!(context, "Cannot add contact.");
            }
        }

        Ok((row_id, sth_modified))
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
    pub async fn add_address_book(context: &Context, addr_book: impl AsRef<str>) -> Result<usize> {
        let mut modify_cnt = 0;

        for (name, addr) in split_address_book(addr_book.as_ref()).into_iter() {
            let (name, addr) = sanitize_name_and_addr(name, addr);
            let name = normalize_name(name);
            match Contact::add_or_lookup(context, name, &addr, Origin::AddressBook).await {
                Err(err) => {
                    warn!(
                        context,
                        "Failed to add address {} from address book: {}", addr, err
                    );
                }
                Ok((_, modified)) => {
                    if modified != Modifier::None {
                        modify_cnt += 1
                    }
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
    /// To get information about a single contact, see dc_get_contact().
    ///
    /// `listflags` is a combination of flags:
    /// - if the flag DC_GCL_ADD_SELF is set, SELF is added to the list unless filtered by other parameters
    /// - if the flag DC_GCL_VERIFIED_ONLY is set, only verified contacts are returned.
    ///   if DC_GCL_VERIFIED_ONLY is not set, verified and unverified contacts are returned.
    /// `query` is a string to filter the list.
    pub async fn get_all(
        context: &Context,
        listflags: u32,
        query: Option<impl AsRef<str>>,
    ) -> Result<Vec<u32>> {
        let self_addr = context
            .get_config(Config::ConfiguredAddr)
            .await
            .unwrap_or_default();

        let mut add_self = false;
        let mut ret = Vec::new();
        let flag_verified_only = (listflags & DC_GCL_VERIFIED_ONLY) != 0;
        let flag_add_self = (listflags & DC_GCL_ADD_SELF) != 0;

        if flag_verified_only || query.is_some() {
            let s3str_like_cmd = format!(
                "%{}%",
                query
                    .as_ref()
                    .map(|s| s.as_ref().to_string())
                    .unwrap_or_default()
            );
            context
                .sql
                .query_map(
                    "SELECT c.id FROM contacts c \
                 LEFT JOIN acpeerstates ps ON c.addr=ps.addr  \
                 WHERE c.addr!=?1 \
                 AND c.id>?2 \
                 AND c.origin>=?3 \
                 AND c.blocked=0 \
                 AND (iif(c.name='',c.authname,c.name) LIKE ?4 OR c.addr LIKE ?5) \
                 AND (1=?6 OR LENGTH(ps.verified_key_fingerprint)!=0)  \
                 ORDER BY LOWER(iif(c.name='',c.authname,c.name)||c.addr),c.id;",
                    paramsv![
                        self_addr,
                        DC_CONTACT_ID_LAST_SPECIAL as i32,
                        Origin::IncomingReplyTo,
                        s3str_like_cmd,
                        s3str_like_cmd,
                        if flag_verified_only { 0i32 } else { 1i32 },
                    ],
                    |row| row.get::<_, i32>(0),
                    |ids| {
                        for id in ids {
                            ret.push(id? as u32);
                        }
                        Ok(())
                    },
                )
                .await?;

            let self_name = context
                .get_config(Config::Displayname)
                .await
                .unwrap_or_default();
            let self_name2 = stock_str::self_msg(context);

            if let Some(query) = query {
                if self_addr.contains(query.as_ref())
                    || self_name.contains(query.as_ref())
                    || self_name2.await.contains(query.as_ref())
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
                    "SELECT id FROM contacts
                 WHERE addr!=?1
                 AND id>?2
                 AND origin>=?3
                 AND blocked=0
                 ORDER BY LOWER(iif(name='',authname,name)||addr),id;",
                    paramsv![self_addr, DC_CONTACT_ID_LAST_SPECIAL as i32, 0x100],
                    |row| row.get::<_, i32>(0),
                    |ids| {
                        for id in ids {
                            ret.push(id? as u32);
                        }
                        Ok(())
                    },
                )
                .await?;
        }

        if flag_add_self && add_self {
            ret.push(DC_CONTACT_ID_SELF);
        }

        Ok(ret)
    }

    pub async fn get_blocked_cnt(context: &Context) -> usize {
        context
            .sql
            .query_get_value::<isize>(
                context,
                "SELECT COUNT(*) FROM contacts WHERE id>? AND blocked!=0",
                paramsv![DC_CONTACT_ID_LAST_SPECIAL as i32],
            )
            .await
            .unwrap_or_default() as usize
    }

    /// Get blocked contacts.
    pub async fn get_all_blocked(context: &Context) -> Vec<u32> {
        context
            .sql
            .query_map(
                "SELECT id FROM contacts WHERE id>? AND blocked!=0 ORDER BY LOWER(name||addr),id;",
                paramsv![DC_CONTACT_ID_LAST_SPECIAL as i32],
                |row| row.get::<_, u32>(0),
                |ids| {
                    ids.collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(Into::into)
                },
            )
            .await
            .unwrap_or_default()
    }

    /// Returns a textual summary of the encryption state for the contact.
    ///
    /// This function returns a string explaining the encryption state
    /// of the contact and if the connection is encrypted the
    /// fingerprints of the keys involved.
    pub async fn get_encrinfo(context: &Context, contact_id: u32) -> Result<String> {
        ensure!(
            contact_id > DC_CONTACT_ID_LAST_SPECIAL,
            "Can not provide encryption info for special contact"
        );

        let mut ret = String::new();
        if let Ok(contact) = Contact::load_from_db(context, contact_id).await {
            let loginparam = LoginParam::from_database(context, "configured_").await;
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

                ret += &format!(
                    "{}\n{}:",
                    stock_message,
                    stock_str::finger_prints(context).await
                );

                let fingerprint_self = SignedPublicKey::load_self(context)
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
                        peerstate.addr.clone(),
                        &fingerprint_other_verified,
                        &fingerprint_other_unverified,
                    );
                } else {
                    cat_fingerprint(
                        &mut ret,
                        peerstate.addr.clone(),
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

    /// Delete a contact. The contact is deleted from the local device. It may happen that this is not
    /// possible as the contact is in use. In this case, the contact can be blocked.
    ///
    /// May result in a `#DC_EVENT_CONTACTS_CHANGED` event.
    pub async fn delete(context: &Context, contact_id: u32) -> Result<()> {
        ensure!(
            contact_id > DC_CONTACT_ID_LAST_SPECIAL,
            "Can not delete special contact"
        );

        let count_contacts: i32 = context
            .sql
            .query_get_value(
                context,
                "SELECT COUNT(*) FROM chats_contacts WHERE contact_id=?;",
                paramsv![contact_id as i32],
            )
            .await
            .unwrap_or_default();

        let count_msgs: i32 = if count_contacts > 0 {
            context
                .sql
                .query_get_value(
                    context,
                    "SELECT COUNT(*) FROM msgs WHERE from_id=? OR to_id=?;",
                    paramsv![contact_id as i32, contact_id as i32],
                )
                .await
                .unwrap_or_default()
        } else {
            0
        };

        if count_msgs == 0 {
            match context
                .sql
                .execute(
                    "DELETE FROM contacts WHERE id=?;",
                    paramsv![contact_id as i32],
                )
                .await
            {
                Ok(_) => {
                    context.emit_event(EventType::ContactsChanged(None));
                    return Ok(());
                }
                Err(err) => {
                    error!(context, "delete_contact {} failed ({})", contact_id, err);
                    return Err(err.into());
                }
            }
        }

        info!(
            context,
            "could not delete contact {}, there are {} messages with it", contact_id, count_msgs
        );
        bail!("Could not delete contact with messages in it");
    }

    /// Get a single contact object.  For a list, see eg. dc_get_contacts().
    ///
    /// For contact DC_CONTACT_ID_SELF (1), the function returns sth.
    /// like "Me" in the selected language and the email address
    /// defined by dc_set_config().
    pub async fn get_by_id(context: &Context, contact_id: u32) -> Result<Contact> {
        let contact = Contact::load_from_db(context, contact_id).await?;

        Ok(contact)
    }

    /// Updates `param` column in the database.
    pub async fn update_param(&self, context: &Context) -> Result<()> {
        context
            .sql
            .execute(
                "UPDATE contacts SET param=? WHERE id=?",
                paramsv![self.param.to_string(), self.id as i32],
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
                paramsv![self.status, self.id as i32],
            )
            .await?;
        Ok(())
    }

    /// Get the ID of the contact.
    pub fn get_id(&self) -> u32 {
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

    /// Get a summary of name and address.
    ///
    /// The returned string is either "Name (email@domain.com)" or just
    /// "email@domain.com" if the name is unset.
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
    /// using dc_set_config(context, "selfavatar", image).
    pub async fn get_profile_image(&self, context: &Context) -> Option<PathBuf> {
        if self.id == DC_CONTACT_ID_SELF {
            if let Some(p) = context.get_config(Config::Selfavatar).await {
                return Some(PathBuf::from(p));
            }
        } else if let Some(image_rel) = self.param.get(Param::ProfileImage) {
            if !image_rel.is_empty() {
                return Some(dc_get_abs_path(context, image_rel));
            }
        }
        None
    }

    /// Get a color for the contact.
    /// The color is calculated from the contact's email address
    /// and can be used for an fallback avatar with white initials
    /// as well as for headlines in bubbles of group chats.
    pub fn get_color(&self) -> u32 {
        str_to_color(&self.addr)
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
    ///
    pub async fn is_verified(&self, context: &Context) -> VerifiedStatus {
        self.is_verified_ex(context, None).await
    }

    /// Same as `Contact::is_verified` but allows speeding up things
    /// by adding the peerstate belonging to the contact.
    /// If you do not have the peerstate available, it is loaded automatically.
    pub async fn is_verified_ex(
        &self,
        context: &Context,
        peerstate: Option<&Peerstate>,
    ) -> VerifiedStatus {
        // We're always sort of secured-verified as we could verify the key on this device any time with the key
        // on this device
        if self.id == DC_CONTACT_ID_SELF {
            return VerifiedStatus::BidirectVerified;
        }

        if let Some(peerstate) = peerstate {
            if peerstate.verified_key.is_some() {
                return VerifiedStatus::BidirectVerified;
            }
        }

        let peerstate = match Peerstate::from_addr(context, &self.addr).await {
            Ok(peerstate) => peerstate,
            Err(err) => {
                warn!(
                    context,
                    "Failed to load peerstate for address {}: {}", self.addr, err
                );
                return VerifiedStatus::Unverified;
            }
        };

        if let Some(ps) = peerstate {
            if ps.verified_key.is_some() {
                return VerifiedStatus::BidirectVerified;
            }
        }

        VerifiedStatus::Unverified
    }

    pub async fn addr_equals_contact(
        context: &Context,
        addr: impl AsRef<str>,
        contact_id: u32,
    ) -> bool {
        if addr.as_ref().is_empty() {
            return false;
        }

        if let Ok(contact) = Contact::load_from_db(context, contact_id).await {
            if !contact.addr.is_empty() {
                let normalized_addr = addr_normalize(addr.as_ref());
                if contact.addr == normalized_addr {
                    return true;
                }
            }
        }

        false
    }

    pub async fn get_real_cnt(context: &Context) -> usize {
        if !context.sql.is_open().await {
            return 0;
        }

        context
            .sql
            .query_get_value::<isize>(
                context,
                "SELECT COUNT(*) FROM contacts WHERE id>?;",
                paramsv![DC_CONTACT_ID_LAST_SPECIAL as i32],
            )
            .await
            .unwrap_or_default() as usize
    }

    pub async fn real_exists_by_id(context: &Context, contact_id: u32) -> bool {
        if !context.sql.is_open().await || contact_id <= DC_CONTACT_ID_LAST_SPECIAL {
            return false;
        }

        context
            .sql
            .exists(
                "SELECT id FROM contacts WHERE id=?;",
                paramsv![contact_id as i32],
            )
            .await
            .unwrap_or_default()
    }

    pub async fn scaleup_origin_by_id(context: &Context, contact_id: u32, origin: Origin) -> bool {
        context
            .sql
            .execute(
                "UPDATE contacts SET origin=? WHERE id=? AND origin<?;",
                paramsv![origin, contact_id as i32, origin],
            )
            .await
            .is_ok()
    }
}

/// Returns false if addr is an invalid address, otherwise true.
pub fn may_be_valid_addr(addr: &str) -> bool {
    let res = addr.parse::<EmailAddress>();
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

pub fn sanitize_name_and_addr(name: impl AsRef<str>, addr: impl AsRef<str>) -> (String, String) {
    static ADDR_WITH_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("(.*)<(.*)>").unwrap());
    if let Some(captures) = ADDR_WITH_NAME_REGEX.captures(addr.as_ref()) {
        (
            if name.as_ref().is_empty() {
                captures
                    .get(1)
                    .map_or("".to_string(), |m| normalize_name(m.as_str()))
            } else {
                name.as_ref().to_string()
            },
            captures
                .get(2)
                .map_or("".to_string(), |m| m.as_str().to_string()),
        )
    } else {
        (name.as_ref().to_string(), addr.as_ref().to_string())
    }
}

async fn set_block_contact(context: &Context, contact_id: u32, new_blocking: bool) {
    if contact_id <= DC_CONTACT_ID_LAST_SPECIAL {
        return;
    }

    if let Ok(contact) = Contact::load_from_db(context, contact_id).await {
        if contact.blocked != new_blocking
            && context
                .sql
                .execute(
                    "UPDATE contacts SET blocked=? WHERE id=?;",
                    paramsv![new_blocking as i32, contact_id as i32],
                )
                .await
                .is_ok()
        {
            // also (un)block all chats with _only_ this contact - we do not delete them to allow a
            // non-destructive blocking->unblocking.
            // (Maybe, beside normal chats (type=100) we should also block group chats with only this user.
            // However, I'm not sure about this point; it may be confusing if the user wants to add other people;
            // this would result in recreating the same group...)
            if context.sql.execute(
                "UPDATE chats SET blocked=? WHERE type=? AND id IN (SELECT chat_id FROM chats_contacts WHERE contact_id=?);",
                paramsv![new_blocking, 100, contact_id as i32]).await.is_ok()
            {
                Contact::mark_noticed(context, contact_id).await;
                context.emit_event(EventType::ContactsChanged(Some(contact_id)));
            }
        }
    }
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
    contact_id: u32,
    profile_image: &AvatarAction,
    was_encrypted: bool,
) -> Result<()> {
    let mut contact = Contact::load_from_db(context, contact_id).await?;
    let changed = match profile_image {
        AvatarAction::Change(profile_image) => {
            if contact_id == DC_CONTACT_ID_SELF {
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
            if contact_id == DC_CONTACT_ID_SELF {
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
pub(crate) async fn set_status(context: &Context, contact_id: u32, status: String) -> Result<()> {
    let mut contact = Contact::load_from_db(context, contact_id).await?;

    if contact.status != status {
        contact.status = status;
        contact.update_status(context).await?;
        context.emit_event(EventType::ContactsChanged(Some(contact_id)));
    }
    Ok(())
}

/// Normalize a name.
///
/// - Remove quotes (come from some bad MUA implementations)
/// - Trims the resulting string
///
/// Typically, this function is not needed as it is called implicitly by `Contact::add_address_book`.
pub fn normalize_name(full_name: impl AsRef<str>) -> String {
    let full_name = full_name.as_ref().trim();
    if full_name.is_empty() {
        return full_name.into();
    }

    match full_name.as_bytes() {
        [b'\'', .., b'\''] | [b'\"', .., b'\"'] | [b'<', .., b'>'] => full_name
            .get(1..full_name.len() - 1)
            .map_or("".to_string(), |s| s.trim().into()),
        _ => full_name.to_string(),
    }
}

fn cat_fingerprint(
    ret: &mut String,
    addr: impl AsRef<str>,
    fingerprint_verified: impl AsRef<str>,
    fingerprint_unverified: impl AsRef<str>,
) {
    *ret += &format!(
        "\n\n{}:\n{}",
        addr.as_ref(),
        if !fingerprint_verified.as_ref().is_empty() {
            fingerprint_verified.as_ref()
        } else {
            fingerprint_unverified.as_ref()
        },
    );
    if !fingerprint_verified.as_ref().is_empty()
        && !fingerprint_unverified.as_ref().is_empty()
        && fingerprint_verified.as_ref() != fingerprint_unverified.as_ref()
    {
        *ret += &format!(
            "\n\n{} (alternative):\n{}",
            addr.as_ref(),
            fingerprint_unverified.as_ref()
        );
    }
}

impl Context {
    /// determine whether the specified addr maps to the/a self addr
    pub async fn is_self_addr(&self, addr: &str) -> Result<bool> {
        let self_addr = self
            .get_config(Config::ConfiguredAddr)
            .await
            .ok_or_else(|| format_err!("Not configured"))?;

        Ok(addr_cmp(self_addr, addr))
    }
}

pub fn addr_cmp(addr1: impl AsRef<str>, addr2: impl AsRef<str>) -> bool {
    let norm1 = addr_normalize(addr1.as_ref()).to_lowercase();
    let norm2 = addr_normalize(addr2.as_ref()).to_lowercase();

    norm1 == norm2
}

fn split_address_book(book: &str) -> Vec<(&str, &str)> {
    book.lines()
        .chunks(2)
        .into_iter()
        .filter_map(|mut chunk| {
            let name = chunk.next().unwrap();
            let addr = match chunk.next() {
                Some(a) => a,
                None => return None,
            };
            Some((name, addr))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::chat::send_text_msg;
    use crate::test_utils::TestContext;

    #[test]
    fn test_may_be_valid_addr() {
        assert_eq!(may_be_valid_addr(""), false);
        assert_eq!(may_be_valid_addr("user@domain.tld"), true);
        assert_eq!(may_be_valid_addr("uuu"), false);
        assert_eq!(may_be_valid_addr("dd.tt"), false);
        assert_eq!(may_be_valid_addr("tt.dd@uu"), false);
        assert_eq!(may_be_valid_addr("u@d"), false);
        assert_eq!(may_be_valid_addr("u@d."), false);
        assert_eq!(may_be_valid_addr("u@d.t"), false);
        assert_eq!(may_be_valid_addr("u@d.tt"), true);
        assert_eq!(may_be_valid_addr("u@.tt"), false);
        assert_eq!(may_be_valid_addr("@d.tt"), false);
        assert_eq!(may_be_valid_addr("<da@d.tt"), false);
        assert_eq!(may_be_valid_addr("sk <@d.tt>"), false);
        assert_eq!(may_be_valid_addr("as@sd.de>"), false);
        assert_eq!(may_be_valid_addr("ask dkl@dd.tt"), false);
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

    #[async_std::test]
    async fn test_get_contacts() -> Result<()> {
        let context = TestContext::new().await;

        // Bob is not in the contacts yet.
        let contacts = Contact::get_all(&context.ctx, 0, Some("bob")).await?;
        assert_eq!(contacts.len(), 0);

        let (id, _modified) = Contact::add_or_lookup(
            &context.ctx,
            "bob",
            "user@example.org",
            Origin::IncomingReplyTo,
        )
        .await?;
        assert_ne!(id, 0);

        let contact = Contact::load_from_db(&context.ctx, id).await.unwrap();
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
            "user@example.org",
            Origin::ManuallyCreated,
        )
        .await?;
        assert_eq!(contact_bob_id, id);
        assert_eq!(modified, Modifier::Modified);
        let contact = Contact::load_from_db(&context.ctx, id).await.unwrap();
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

    #[async_std::test]
    async fn test_is_self_addr() -> Result<()> {
        let t = TestContext::new().await;
        assert!(t.is_self_addr("me@me.org").await.is_err());

        let addr = t.configure_alice().await;
        assert_eq!(t.is_self_addr("me@me.org").await?, false);
        assert_eq!(t.is_self_addr(&addr).await?, true);

        Ok(())
    }

    #[async_std::test]
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

        // check first added contact, this modifies authname beacuse it is empty
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t, "bla foo", "one@eins.org", Origin::IncomingUnknownTo)
                .await
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_id(), contact_id);
        assert_eq!(contact.get_name(), "Name one");
        assert_eq!(contact.get_authname(), "bla foo");
        assert_eq!(contact.get_display_name(), "Name one");
        assert_eq!(contact.get_addr(), "one@eins.org");
        assert_eq!(contact.get_name_n_addr(), "Name one (one@eins.org)");

        // modify first added contact
        let (contact_id_test, sth_modified) =
            Contact::add_or_lookup(&t, "Real one", " one@eins.org  ", Origin::ManuallyCreated)
                .await
                .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "Real one");
        assert_eq!(contact.get_addr(), "one@eins.org");
        assert!(!contact.is_blocked());

        // check third added contact (contact without name)
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t, "", "three@drei.sam", Origin::IncomingUnknownTo)
                .await
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::None);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "three@drei.sam");
        assert_eq!(contact.get_addr(), "three@drei.sam");
        assert_eq!(contact.get_name_n_addr(), "three@drei.sam");

        // add name to third contact from incoming message (this becomes authorized name)
        let (contact_id_test, sth_modified) = Contact::add_or_lookup(
            &t,
            "m. serious",
            "three@drei.sam",
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name_n_addr(), "m. serious (three@drei.sam)");
        assert!(!contact.is_blocked());

        // manually edit name of third contact (does not changed authorized name)
        let (contact_id_test, sth_modified) =
            Contact::add_or_lookup(&t, "schnucki", "three@drei.sam", Origin::ManuallyCreated)
                .await
                .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "m. serious");
        assert_eq!(contact.get_name_n_addr(), "schnucki (three@drei.sam)");
        assert!(!contact.is_blocked());

        // Fourth contact:
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t, "", "alice@w.de", Origin::IncomingUnknownTo)
                .await
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::None);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "Wonderland, Alice");
        assert_eq!(contact.get_display_name(), "Wonderland, Alice");
        assert_eq!(contact.get_addr(), "alice@w.de");
        assert_eq!(contact.get_name_n_addr(), "Wonderland, Alice (alice@w.de)");

        // check SELF
        let contact = Contact::load_from_db(&t, DC_CONTACT_ID_SELF).await.unwrap();
        assert_eq!(DC_CONTACT_ID_SELF, 1);
        assert_eq!(contact.get_name(), stock_str::self_msg(&t).await);
        assert_eq!(contact.get_addr(), ""); // we're not configured
        assert!(!contact.is_blocked());
    }

    #[async_std::test]
    async fn test_remote_authnames() {
        let t = TestContext::new().await;

        // incoming mail `From: bob1 <bob@example.org>` - this should init authname
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t, "bob1", "bob@example.org", Origin::IncomingUnknownFrom)
                .await
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::Created);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob1");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "bob1");

        // incoming mail `From: bob2 <bob@example.org>` - this should update authname
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t, "bob2", "bob@example.org", Origin::IncomingUnknownFrom)
                .await
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob2");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "bob2");

        // manually edit name to "bob3" - authname should be still be "bob2" as given in `From:` above
        let contact_id = Contact::create(&t, "bob3", "bob@example.org")
            .await
            .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob2");
        assert_eq!(contact.get_name(), "bob3");
        assert_eq!(contact.get_display_name(), "bob3");

        // incoming mail `From: bob4 <bob@example.org>` - this should update authname, manually given name is still "bob3"
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t, "bob4", "bob@example.org", Origin::IncomingUnknownFrom)
                .await
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "bob4");
        assert_eq!(contact.get_name(), "bob3");
        assert_eq!(contact.get_display_name(), "bob3");
    }

    #[async_std::test]
    async fn test_remote_authnames_create_empty() {
        let t = TestContext::new().await;

        // manually create "claire@example.org" without a given name
        let contact_id = Contact::create(&t, "", "claire@example.org").await.unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "claire@example.org");

        // incoming mail `From: claire1 <claire@example.org>` - this should update authname
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t,
            "claire1",
            "claire@example.org",
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "claire1");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "claire1");

        // incoming mail `From: claire2 <claire@example.org>` - this should update authname
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t,
            "claire2",
            "claire@example.org",
            Origin::IncomingUnknownFrom,
        )
        .await
        .unwrap();
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "claire2");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "claire2");
    }

    /// Regression test.
    ///
    /// In the past, "Not Bob" name was stuck until "Bob" changed the name to "Not Bob" and back in
    /// the "From:" field or user set the name to empty string manually.
    #[async_std::test]
    async fn test_remote_authnames_update_to() -> Result<()> {
        let t = TestContext::new().await;

        // Incoming message from Bob.
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t, "Bob", "bob@example.org", Origin::IncomingUnknownFrom)
                .await?;
        assert_eq!(sth_modified, Modifier::Created);
        let contact = Contact::load_from_db(&t, contact_id).await?;
        assert_eq!(contact.get_display_name(), "Bob");

        // Incoming message from someone else with "Not Bob" <bob@example.org> in the "To:" field.
        let (contact_id_same, sth_modified) =
            Contact::add_or_lookup(&t, "Not Bob", "bob@example.org", Origin::IncomingUnknownTo)
                .await?;
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t, contact_id).await?;
        assert_eq!(contact.get_display_name(), "Not Bob");

        // Incoming message from Bob, changing the name back.
        let (contact_id_same, sth_modified) =
            Contact::add_or_lookup(&t, "Bob", "bob@example.org", Origin::IncomingUnknownFrom)
                .await?;
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified); // This was None until the bugfix
        let contact = Contact::load_from_db(&t, contact_id).await?;
        assert_eq!(contact.get_display_name(), "Bob");

        Ok(())
    }

    #[async_std::test]
    async fn test_remote_authnames_edit_empty() {
        let t = TestContext::new().await;

        // manually create "dave@example.org"
        let contact_id = Contact::create(&t, "dave1", "dave@example.org")
            .await
            .unwrap();
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "");
        assert_eq!(contact.get_name(), "dave1");
        assert_eq!(contact.get_display_name(), "dave1");

        // incoming mail `From: dave2 <dave@example.org>` - this should update authname
        Contact::add_or_lookup(&t, "dave2", "dave@example.org", Origin::IncomingUnknownFrom)
            .await
            .unwrap();
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_authname(), "dave2");
        assert_eq!(contact.get_name(), "dave1");
        assert_eq!(contact.get_display_name(), "dave1");

        // manually clear the name
        Contact::create(&t, "", "dave@example.org").await.unwrap();
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
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

    #[async_std::test]
    async fn test_name_in_address() {
        let t = TestContext::new().await;

        let contact_id = Contact::create(&t, "", "<dave@example.org>").await.unwrap();
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_addr(), "dave@example.org");

        let contact_id = Contact::create(&t, "", "Mueller, Dave <dave@example.org>")
            .await
            .unwrap();
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "Mueller, Dave");
        assert_eq!(contact.get_addr(), "dave@example.org");

        let contact_id = Contact::create(&t, "name1", "name2 <dave@example.org>")
            .await
            .unwrap();
        let contact = Contact::load_from_db(&t, contact_id).await.unwrap();
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
        assert!(Contact::create(&t, "", "dskjf@dslk@sadkljdk")
            .await
            .is_err());
        assert!(Contact::create(&t, "", "dskjf dslk@d.e").await.is_err());
        assert!(Contact::create(&t, "", "<dskjf dslk@sadklj.dk")
            .await
            .is_err());
    }

    #[async_std::test]
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

        let id = Contact::lookup_id_by_addr(&alice.ctx, "alice@example.com", Origin::Unknown)
            .await
            .unwrap();
        assert_eq!(id, Some(DC_CONTACT_ID_SELF));
    }

    #[async_std::test]
    async fn test_contact_get_encrinfo() -> Result<()> {
        let alice = TestContext::new_alice().await;

        // Return error for special IDs
        let encrinfo = Contact::get_encrinfo(&alice, DC_CONTACT_ID_SELF).await;
        assert!(encrinfo.is_err());
        let encrinfo = Contact::get_encrinfo(&alice, DC_CONTACT_ID_DEVICE).await;
        assert!(encrinfo.is_err());

        let (contact_bob_id, _modified) =
            Contact::add_or_lookup(&alice, "Bob", "bob@example.net", Origin::ManuallyCreated)
                .await?;

        let encrinfo = Contact::get_encrinfo(&alice, contact_bob_id).await?;
        assert_eq!(encrinfo, "No encryption.");

        let bob = TestContext::new_bob().await;
        let chat_alice = bob
            .create_chat_with_contact("Alice", "alice@example.com")
            .await;
        send_text_msg(&bob, chat_alice.id, "Hello".to_string()).await?;
        let msg = bob.pop_sent_msg().await;
        alice.recv_msg(&msg).await;

        let encrinfo = Contact::get_encrinfo(&alice, contact_bob_id).await?;
        assert_eq!(
            encrinfo,
            "End-to-end encryption preferred.
Fingerprints:

alice@example.com:
2E6F A2CB 23B5 32D7 2863
4B58 64B0 8F61 A9ED 9443

bob@example.net:
CCCB 5AA9 F6E1 141C 9431
65F1 DB18 B18C BCF7 0487"
        );

        Ok(())
    }
}
