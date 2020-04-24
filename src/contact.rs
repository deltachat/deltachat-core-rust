//! Contacts module

use std::path::PathBuf;

use deltachat_derive::*;
use itertools::Itertools;

use crate::aheader::EncryptPreference;
use crate::chat::ChatId;
use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::{bail, ensure, format_err, Result};
use crate::events::Event;
use crate::key::{DcKey, Key, SignedPublicKey};
use crate::login_param::{LoginParam, ServerSecurity, Service};
use crate::message::{MessageState, MsgId};
use crate::mimeparser::AvatarAction;
use crate::param::*;
use crate::peerstate::*;
use crate::sql;
use crate::stock::StockMessage;

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
}

/// Possible origins of a contact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromPrimitive, ToPrimitive, FromSql, ToSql,
)]
#[repr(i32)]
pub enum Origin {
    Unknown = 0,

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
    pub fn load_from_db(context: &Context, contact_id: u32) -> crate::sql::Result<Self> {
        let mut res = context.sql.query_row(
            "SELECT c.name, c.addr, c.origin, c.blocked, c.authname, c.param
               FROM contacts c
              WHERE c.id=?;",
            params![contact_id as i32],
            |row| {
                let contact = Self {
                    id: contact_id,
                    name: row.get::<_, String>(0)?,
                    authname: row.get::<_, String>(4)?,
                    addr: row.get::<_, String>(1)?,
                    blocked: row.get::<_, Option<i32>>(3)?.unwrap_or_default() != 0,
                    origin: row.get(2)?,
                    param: row.get::<_, String>(5)?.parse().unwrap_or_default(),
                };
                Ok(contact)
            },
        )?;
        if contact_id == DC_CONTACT_ID_SELF {
            res.name = context.stock_str(StockMessage::SelfMsg).to_string();
            res.addr = context
                .get_config(Config::ConfiguredAddr)
                .unwrap_or_default();
        } else if contact_id == DC_CONTACT_ID_DEVICE {
            res.name = context.stock_str(StockMessage::DeviceMessages).to_string();
            res.addr = DC_CONTACT_ID_DEVICE_ADDR.to_string();
        }
        Ok(res)
    }

    /// Returns `true` if this contact is blocked.
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Check if a contact is blocked.
    pub fn is_blocked_load(context: &Context, id: u32) -> bool {
        Self::load_from_db(context, id)
            .map(|contact| contact.blocked)
            .unwrap_or_default()
    }

    /// Block the given contact.
    pub fn block(context: &Context, id: u32) {
        set_block_contact(context, id, true);
    }

    /// Unblock the given contact.
    pub fn unblock(context: &Context, id: u32) {
        set_block_contact(context, id, false);
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
    pub fn create(context: &Context, name: impl AsRef<str>, addr: impl AsRef<str>) -> Result<u32> {
        ensure!(
            !addr.as_ref().is_empty(),
            "Cannot create contact with empty address"
        );

        let (contact_id, sth_modified) =
            Contact::add_or_lookup(context, name, addr, Origin::ManuallyCreated)?;
        let blocked = Contact::is_blocked_load(context, contact_id);
        context.call_cb(Event::ContactsChanged(
            if sth_modified == Modifier::Created {
                Some(contact_id)
            } else {
                None
            },
        ));
        if blocked {
            Contact::unblock(context, contact_id);
        }

        Ok(contact_id)
    }

    /// Mark all messages sent by the given contact
    /// as *noticed*.  See also dc_marknoticed_chat() and dc_markseen_msgs()
    ///
    /// Calling this function usually results in the event `#DC_EVENT_MSGS_CHANGED`.
    pub fn mark_noticed(context: &Context, id: u32) {
        if sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET state=? WHERE from_id=? AND state=?;",
            params![MessageState::InNoticed, id as i32, MessageState::InFresh],
        )
        .is_ok()
        {
            context.call_cb(Event::MsgsChanged {
                chat_id: ChatId::new(0),
                msg_id: MsgId::new(0),
            });
        }
    }

    /// Check if an e-mail address belongs to a known and unblocked contact.
    /// Known and unblocked contacts will be returned by `dc_get_contacts()`.
    ///
    /// To validate an e-mail address independently of the contact database
    /// use `dc_may_be_valid_addr()`.
    pub fn lookup_id_by_addr(context: &Context, addr: impl AsRef<str>, min_origin: Origin) -> u32 {
        if addr.as_ref().is_empty() {
            return 0;
        }

        let addr_normalized = addr_normalize(addr.as_ref());
        let addr_self = context
            .get_config(Config::ConfiguredAddr)
            .unwrap_or_default();

        if addr_cmp(addr_normalized, addr_self) {
            return DC_CONTACT_ID_SELF;
        }
        context.sql.query_get_value(
            context,
            "SELECT id FROM contacts WHERE addr=?1 COLLATE NOCASE AND id>?2 AND origin>=?3 AND blocked=0;",
            params![
                addr_normalized,
                DC_CONTACT_ID_LAST_SPECIAL as i32,
                min_origin as u32,
            ],
        ).unwrap_or_default()
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
    pub(crate) fn add_or_lookup(
        context: &Context,
        name: impl AsRef<str>,
        addr: impl AsRef<str>,
        origin: Origin,
    ) -> Result<(u32, Modifier)> {
        let mut sth_modified = Modifier::None;

        ensure!(
            !addr.as_ref().is_empty(),
            "Can not add_or_lookup empty address"
        );
        ensure!(origin != Origin::Unknown, "Missing valid origin");

        let addr = addr_normalize(addr.as_ref());
        let addr_self = context
            .get_config(Config::ConfiguredAddr)
            .unwrap_or_default();

        if addr_cmp(addr, addr_self) {
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

        let mut update_addr = false;
        let mut update_name = false;
        let mut update_authname = false;
        let mut row_id = 0;

        if let Ok((id, row_name, row_addr, row_origin, row_authname)) = context.sql.query_row(
            "SELECT id, name, addr, origin, authname FROM contacts WHERE addr=? COLLATE NOCASE;",
            params![addr],
            |row| {
                let row_id = row.get(0)?;
                let row_name: String = row.get(1)?;
                let row_addr: String = row.get(2)?;
                let row_origin: Origin = row.get(3)?;
                let row_authname: String = row.get(4)?;

                if !name.as_ref().is_empty() {
                    if !row_name.is_empty() {
                        if (origin >= row_origin || row_name == row_authname)
                            && name.as_ref() != row_name
                        {
                            update_name = true;
                        }
                    } else {
                        update_name = true;
                    }
                    if origin == Origin::IncomingUnknownFrom && name.as_ref() != row_authname {
                        update_authname = true;
                    }
                } else if origin == Origin::ManuallyCreated && !row_authname.is_empty() {
                    // no name given on manual edit, this will update the name to the authname
                    update_name = true;
                }

                Ok((row_id, row_name, row_addr, row_origin, row_authname))
            },
        ) {
            row_id = id;
            if origin as i32 >= row_origin as i32 && addr != row_addr {
                update_addr = true;
            }
            if update_name || update_authname || update_addr || origin > row_origin {
                let new_name = if update_name {
                    if !name.as_ref().is_empty() {
                        name.as_ref()
                    } else {
                        &row_authname
                    }
                } else {
                    &row_name
                };

                sql::execute(
                    context,
                    &context.sql,
                    "UPDATE contacts SET name=?, addr=?, origin=?, authname=? WHERE id=?;",
                    params![
                        new_name,
                        if update_addr { addr } else { &row_addr },
                        if origin > row_origin {
                            origin
                        } else {
                            row_origin
                        },
                        if update_authname {
                            name.as_ref()
                        } else {
                            &row_authname
                        },
                        row_id
                    ],
                )
                .ok();

                if update_name {
                    // Update the contact name also if it is used as a group name.
                    // This is one of the few duplicated data, however, getting the chat list is easier this way.
                    sql::execute(
                    context,
                    &context.sql,
                    "UPDATE chats SET name=? WHERE type=? AND id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?);",
                    params![new_name, Chattype::Single, row_id]
                ).ok();
                }
                sth_modified = Modifier::Modified;
            }
        } else {
            if origin == Origin::IncomingUnknownFrom {
                update_authname = true;
            }

            if sql::execute(
                context,
                &context.sql,
                "INSERT INTO contacts (name, addr, origin, authname) VALUES(?, ?, ?, ?);",
                params![
                    name.as_ref(),
                    addr,
                    origin,
                    if update_authname { name.as_ref() } else { "" }
                ],
            )
            .is_ok()
            {
                row_id = sql::get_rowid(context, &context.sql, "contacts", "addr", addr);
                sth_modified = Modifier::Created;
                info!(context, "added contact id={} addr={}", row_id, addr);
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
    pub fn add_address_book(context: &Context, addr_book: impl AsRef<str>) -> Result<usize> {
        let mut modify_cnt = 0;

        for (name, addr) in split_address_book(addr_book.as_ref()).into_iter() {
            let name = normalize_name(name);
            match Contact::add_or_lookup(context, name, addr, Origin::AddressBook) {
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
            context.call_cb(Event::ContactsChanged(None));
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
    pub fn get_all(
        context: &Context,
        listflags: u32,
        query: Option<impl AsRef<str>>,
    ) -> Result<Vec<u32>> {
        let self_addr = context
            .get_config(Config::ConfiguredAddr)
            .unwrap_or_default();

        let mut add_self = false;
        let mut ret = Vec::new();
        let flag_verified_only = listflags_has(listflags, DC_GCL_VERIFIED_ONLY);
        let flag_add_self = listflags_has(listflags, DC_GCL_ADD_SELF);

        if flag_verified_only || query.is_some() {
            let s3str_like_cmd = format!(
                "%{}%",
                query
                    .as_ref()
                    .map(|s| s.as_ref().to_string())
                    .unwrap_or_default()
            );
            context.sql.query_map(
                "SELECT c.id FROM contacts c \
                 LEFT JOIN acpeerstates ps ON c.addr=ps.addr  \
                 WHERE c.addr!=?1 \
                 AND c.id>?2 \
                 AND c.origin>=?3 \
                 AND c.blocked=0 \
                 AND (c.name LIKE ?4 OR c.addr LIKE ?5) \
                 AND (1=?6 OR LENGTH(ps.verified_key_fingerprint)!=0)  \
                 ORDER BY LOWER(c.name||c.addr),c.id;",
                params![
                    self_addr,
                    DC_CONTACT_ID_LAST_SPECIAL as i32,
                    Origin::IncomingReplyTo,
                    &s3str_like_cmd,
                    &s3str_like_cmd,
                    if flag_verified_only { 0 } else { 1 },
                ],
                |row| row.get::<_, i32>(0),
                |ids| {
                    for id in ids {
                        ret.push(id? as u32);
                    }
                    Ok(())
                },
            )?;

            let self_name = context.get_config(Config::Displayname).unwrap_or_default();
            let self_name2 = context.stock_str(StockMessage::SelfMsg);

            if let Some(query) = query {
                if self_addr.contains(query.as_ref())
                    || self_name.contains(query.as_ref())
                    || self_name2.contains(query.as_ref())
                {
                    add_self = true;
                }
            } else {
                add_self = true;
            }
        } else {
            add_self = true;

            context.sql.query_map(
                "SELECT id FROM contacts WHERE addr!=?1 AND id>?2 AND origin>=?3 AND blocked=0 ORDER BY LOWER(name||addr),id;",
                params![self_addr, DC_CONTACT_ID_LAST_SPECIAL as i32, 0x100],
                |row| row.get::<_, i32>(0),
                |ids| {
                    for id in ids {
                        ret.push(id? as u32);
                    }
                    Ok(())
                }
            )?;
        }

        if flag_add_self && add_self {
            ret.push(DC_CONTACT_ID_SELF);
        }

        Ok(ret)
    }

    pub fn get_blocked_cnt(context: &Context) -> usize {
        context
            .sql
            .query_get_value::<_, isize>(
                context,
                "SELECT COUNT(*) FROM contacts WHERE id>? AND blocked!=0",
                params![DC_CONTACT_ID_LAST_SPECIAL as i32],
            )
            .unwrap_or_default() as usize
    }

    /// Get blocked contacts.
    pub fn get_all_blocked(context: &Context) -> Vec<u32> {
        context
            .sql
            .query_map(
                "SELECT id FROM contacts WHERE id>? AND blocked!=0 ORDER BY LOWER(name||addr),id;",
                params![DC_CONTACT_ID_LAST_SPECIAL as i32],
                |row| row.get::<_, u32>(0),
                |ids| {
                    ids.collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(Into::into)
                },
            )
            .unwrap_or_default()
    }

    /// Returns a textual summary of the encryption state for the contact.
    ///
    /// This function returns a string explaining the encryption state
    /// of the contact and if the connection is encrypted the
    /// fingerprints of the keys involved.
    pub fn get_encrinfo(context: &Context, contact_id: u32) -> Result<String> {
        let mut ret = String::new();

        if let Ok(contact) = Contact::load_from_db(context, contact_id) {
            let peerstate = Peerstate::from_addr(context, &context.sql, &contact.addr);
            let loginparam = LoginParam::from_database(context, "configured_");

            if peerstate.is_some()
                && peerstate
                    .as_ref()
                    .and_then(|p| p.peek_key(PeerstateVerifiedStatus::Unverified))
                    .is_some()
            {
                let peerstate = peerstate.as_ref().unwrap();
                let p =
                    context.stock_str(if peerstate.prefer_encrypt == EncryptPreference::Mutual {
                        StockMessage::E2ePreferred
                    } else {
                        StockMessage::E2eAvailable
                    });
                ret += &p;
                let self_key = Key::from(SignedPublicKey::load_self(context)?);
                let p = context.stock_str(StockMessage::FingerPrints);
                ret += &format!(" {}:", p);

                let fingerprint_self = self_key.formatted_fingerprint();
                let fingerprint_other_verified = peerstate
                    .peek_key(PeerstateVerifiedStatus::BidirectVerified)
                    .map(|k| k.formatted_fingerprint())
                    .unwrap_or_default();
                let fingerprint_other_unverified = peerstate
                    .peek_key(PeerstateVerifiedStatus::Unverified)
                    .map(|k| k.formatted_fingerprint())
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
            } else if Some(ServerSecurity::PlainSocket)
                == loginparam.srv_params[Service::Imap as usize].security
                && Some(ServerSecurity::PlainSocket)
                    == loginparam.srv_params[Service::Smtp as usize].security
            {
                ret += &context.stock_str(StockMessage::EncrTransp);
            } else {
                ret += &context.stock_str(StockMessage::EncrNone);
            }
        }

        Ok(ret)
    }

    /// Delete a contact. The contact is deleted from the local device. It may happen that this is not
    /// possible as the contact is in use. In this case, the contact can be blocked.
    ///
    /// May result in a `#DC_EVENT_CONTACTS_CHANGED` event.
    pub fn delete(context: &Context, contact_id: u32) -> Result<()> {
        ensure!(
            contact_id > DC_CONTACT_ID_LAST_SPECIAL,
            "Can not delete special contact"
        );

        let count_contacts: i32 = context
            .sql
            .query_get_value(
                context,
                "SELECT COUNT(*) FROM chats_contacts WHERE contact_id=?;",
                params![contact_id as i32],
            )
            .unwrap_or_default();

        let count_msgs: i32 = if count_contacts > 0 {
            context
                .sql
                .query_get_value(
                    context,
                    "SELECT COUNT(*) FROM msgs WHERE from_id=? OR to_id=?;",
                    params![contact_id as i32, contact_id as i32],
                )
                .unwrap_or_default()
        } else {
            0
        };

        if count_msgs == 0 {
            match sql::execute(
                context,
                &context.sql,
                "DELETE FROM contacts WHERE id=?;",
                params![contact_id as i32],
            ) {
                Ok(_) => {
                    context.call_cb(Event::ContactsChanged(None));
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
    pub fn get_by_id(context: &Context, contact_id: u32) -> Result<Contact> {
        Ok(Contact::load_from_db(context, contact_id)?)
    }

    pub fn update_param(&mut self, context: &Context) -> Result<()> {
        sql::execute(
            context,
            &context.sql,
            "UPDATE contacts SET param=? WHERE id=?",
            params![self.param.to_string(), self.id as i32],
        )?;
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

    /// Get the contact name. This is the name as defined by the contact himself or
    /// modified by the user.  May be an empty string.
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
            return format!("{} ({})", self.name, self.addr);
        }
        (&self.addr).into()
    }

    /// Get the part of the name before the first space. In most languages, this seems to be
    /// the prename. If there is no space, the full display name is returned.
    /// If the display name is not set, the e-mail address is returned.
    pub fn get_first_name(&self) -> &str {
        if !self.name.is_empty() {
            return get_first_name(&self.name);
        }
        &self.addr
    }

    /// Get the contact's profile image.
    /// This is the image set by each remote user on their own
    /// using dc_set_config(context, "selfavatar", image).
    pub fn get_profile_image(&self, context: &Context) -> Option<PathBuf> {
        if self.id == DC_CONTACT_ID_SELF {
            if let Some(p) = context.get_config(Config::Selfavatar) {
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
        dc_str_to_color(&self.addr)
    }

    /// Check if a contact was verified. E.g. by a secure-join QR code scan
    /// and if the key has not changed since this verification.
    ///
    /// The UI may draw a checkbox or something like that beside verified contacts.
    ///
    pub fn is_verified(&self, context: &Context) -> VerifiedStatus {
        self.is_verified_ex(context, None)
    }

    /// Same as `Contact::is_verified` but allows speeding up things
    /// by adding the peerstate belonging to the contact.
    /// If you do not have the peerstate available, it is loaded automatically.
    pub fn is_verified_ex(
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

        let peerstate = Peerstate::from_addr(context, &context.sql, &self.addr);
        if let Some(ps) = peerstate {
            if ps.verified_key.is_some() {
                return VerifiedStatus::BidirectVerified;
            }
        }

        VerifiedStatus::Unverified
    }

    pub fn addr_equals_contact(context: &Context, addr: impl AsRef<str>, contact_id: u32) -> bool {
        if addr.as_ref().is_empty() {
            return false;
        }

        if let Ok(contact) = Contact::load_from_db(context, contact_id) {
            if !contact.addr.is_empty() {
                let normalized_addr = addr_normalize(addr.as_ref());
                if contact.addr == normalized_addr {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_real_cnt(context: &Context) -> usize {
        if !context.sql.is_open() {
            return 0;
        }

        context
            .sql
            .query_get_value::<_, isize>(
                context,
                "SELECT COUNT(*) FROM contacts WHERE id>?;",
                params![DC_CONTACT_ID_LAST_SPECIAL as i32],
            )
            .unwrap_or_default() as usize
    }

    pub fn real_exists_by_id(context: &Context, contact_id: u32) -> bool {
        if !context.sql.is_open() || contact_id <= DC_CONTACT_ID_LAST_SPECIAL {
            return false;
        }

        context
            .sql
            .exists(
                "SELECT id FROM contacts WHERE id=?;",
                params![contact_id as i32],
            )
            .unwrap_or_default()
    }

    pub fn scaleup_origin_by_id(context: &Context, contact_id: u32, origin: Origin) -> bool {
        context
            .sql
            .execute(
                "UPDATE contacts SET origin=? WHERE id=? AND origin<?;",
                params![origin, contact_id as i32, origin],
            )
            .is_ok()
    }
}

/// Extracts first name from full name.
fn get_first_name(full_name: &str) -> &str {
    full_name.splitn(2, ' ').next().unwrap_or_default()
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
        return &norm[7..];
    }

    norm
}

fn set_block_contact(context: &Context, contact_id: u32, new_blocking: bool) {
    if contact_id <= DC_CONTACT_ID_LAST_SPECIAL {
        return;
    }

    if let Ok(contact) = Contact::load_from_db(context, contact_id) {
        if contact.blocked != new_blocking
            && sql::execute(
                context,
                &context.sql,
                "UPDATE contacts SET blocked=? WHERE id=?;",
                params![new_blocking as i32, contact_id as i32],
            )
            .is_ok()
        {
            // also (un)block all chats with _only_ this contact - we do not delete them to allow a
            // non-destructive blocking->unblocking.
            // (Maybe, beside normal chats (type=100) we should also block group chats with only this user.
            // However, I'm not sure about this point; it may be confusing if the user wants to add other people;
            // this would result in recreating the same group...)
            if sql::execute(
                    context,
                    &context.sql,
                    "UPDATE chats SET blocked=? WHERE type=? AND id IN (SELECT chat_id FROM chats_contacts WHERE contact_id=?);",
                    params![new_blocking, 100, contact_id as i32],
                ).is_ok() {
                    Contact::mark_noticed(context, contact_id);
                    context.call_cb(Event::ContactsChanged(None));
                }
        }
    }
}

pub(crate) fn set_profile_image(
    context: &Context,
    contact_id: u32,
    profile_image: &AvatarAction,
) -> Result<()> {
    // the given profile image is expected to be already in the blob directory
    // as profile images can be set only by receiving messages, this should be always the case, however.
    let mut contact = Contact::load_from_db(context, contact_id)?;
    let changed = match profile_image {
        AvatarAction::Change(profile_image) => {
            contact.param.set(Param::ProfileImage, profile_image);
            true
        }
        AvatarAction::Delete => {
            contact.param.remove(Param::ProfileImage);
            true
        }
    };
    if changed {
        contact.update_param(context)?;
        context.call_cb(Event::ContactsChanged(Some(contact_id)));
    }
    Ok(())
}

/// Normalize a name.
///
/// - Remove quotes (come from some bad MUA implementations)
/// - Convert names as "Petersen, Björn" to "Björn Petersen"
/// - Trims the resulting string
///
/// Typically, this function is not needed as it is called implicitly by `Contact::add_address_book`.
pub fn normalize_name(full_name: impl AsRef<str>) -> String {
    let mut full_name = full_name.as_ref().trim();
    if full_name.is_empty() {
        return full_name.into();
    }

    let len = full_name.len();
    if len > 1 {
        let firstchar = full_name.as_bytes()[0];
        let lastchar = full_name.as_bytes()[len - 1];
        if firstchar == b'\'' && lastchar == b'\''
            || firstchar == b'\"' && lastchar == b'\"'
            || firstchar == b'<' && lastchar == b'>'
        {
            full_name = &full_name[1..len - 1];
        }
    }

    if let Some(p1) = full_name.find(',') {
        let (last_name, first_name) = full_name.split_at(p1);

        let last_name = last_name.trim();
        let first_name = (&first_name[1..]).trim();

        return format!("{} {}", first_name, last_name);
    }

    full_name.trim().into()
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
    pub fn is_self_addr(&self, addr: &str) -> Result<bool> {
        let self_addr = self
            .get_config(Config::ConfiguredAddr)
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

    use crate::test_utils::*;

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
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(&normalize_name("Doe, John"), "John Doe");
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
    fn test_get_first_name() {
        assert_eq!(get_first_name("John Doe"), "John");
    }

    #[test]
    fn test_split_address_book() {
        let book = "Name one\nAddress one\nName two\nAddress two\nrest name";
        let list = split_address_book(&book);
        assert_eq!(
            list,
            vec![("Name one", "Address one"), ("Name two", "Address two")]
        )
    }

    #[test]
    fn test_get_contacts() {
        let context = dummy_context();
        let contacts = Contact::get_all(&context.ctx, 0, Some("some2")).unwrap();
        assert_eq!(contacts.len(), 0);

        let id = Contact::create(&context.ctx, "bob", "bob@mail.de").unwrap();
        assert_ne!(id, 0);

        let contacts = Contact::get_all(&context.ctx, 0, Some("bob")).unwrap();
        assert_eq!(contacts.len(), 1);

        let contacts = Contact::get_all(&context.ctx, 0, Some("alice")).unwrap();
        assert_eq!(contacts.len(), 0);
    }

    #[test]
    fn test_is_self_addr() -> Result<()> {
        let t = test_context(None);
        assert!(t.ctx.is_self_addr("me@me.org").is_err());

        let addr = configure_alice_keypair(&t.ctx);
        assert_eq!(t.ctx.is_self_addr("me@me.org")?, false);
        assert_eq!(t.ctx.is_self_addr(&addr)?, true);

        Ok(())
    }

    #[test]
    fn test_add_or_lookup() {
        // add some contacts, this also tests add_address_book()
        let t = dummy_context();
        let book = concat!(
            "  Name one  \n one@eins.org \n",
            "Name two\ntwo@deux.net\n",
            "Invalid\n+1234567890\n", // invalid, should be ignored
            "\nthree@drei.sam\n",
            "Name two\ntwo@deux.net\n" // should not be added again
        );
        assert_eq!(Contact::add_address_book(&t.ctx, book).unwrap(), 3);

        // check first added contact, this does not modify because of lower origin
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t.ctx, "bla foo", "one@eins.org", Origin::IncomingUnknownTo)
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::None);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_id(), contact_id);
        assert_eq!(contact.get_name(), "Name one");
        assert_eq!(contact.get_display_name(), "Name one");
        assert_eq!(contact.get_addr(), "one@eins.org");
        assert_eq!(contact.get_name_n_addr(), "Name one (one@eins.org)");

        // modify first added contact
        let (contact_id_test, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "Real one",
            " one@eins.org  ",
            Origin::ManuallyCreated,
        )
        .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_name(), "Real one");
        assert_eq!(contact.get_addr(), "one@eins.org");
        assert!(!contact.is_blocked());

        // check third added contact (contact without name)
        let (contact_id, sth_modified) =
            Contact::add_or_lookup(&t.ctx, "", "three@drei.sam", Origin::IncomingUnknownTo)
                .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::None);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "three@drei.sam");
        assert_eq!(contact.get_addr(), "three@drei.sam");
        assert_eq!(contact.get_name_n_addr(), "three@drei.sam");

        // add name to third contact from incoming message (this becomes authorized name)
        let (contact_id_test, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "m. serious",
            "three@drei.sam",
            Origin::IncomingUnknownFrom,
        )
        .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_name_n_addr(), "m. serious (three@drei.sam)");
        assert!(!contact.is_blocked());

        // manually edit name of third contact (does not changed authorized name)
        let (contact_id_test, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "schnucki",
            "three@drei.sam",
            Origin::ManuallyCreated,
        )
        .unwrap();
        assert_eq!(contact_id, contact_id_test);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "m. serious");
        assert_eq!(contact.get_name_n_addr(), "schnucki (three@drei.sam)");
        assert!(!contact.is_blocked());

        // check SELF
        let contact = Contact::load_from_db(&t.ctx, DC_CONTACT_ID_SELF).unwrap();
        assert_eq!(DC_CONTACT_ID_SELF, 1);
        assert_eq!(contact.get_name(), t.ctx.stock_str(StockMessage::SelfMsg));
        assert_eq!(contact.get_addr(), ""); // we're not configured
        assert!(!contact.is_blocked());
    }

    #[test]
    fn test_remote_authnames() {
        let t = dummy_context();

        // incoming mail `From: bob1 <bob@example.org>` - this should init authname and name
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "bob1",
            "bob@example.org",
            Origin::IncomingUnknownFrom,
        )
        .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::Created);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "bob1");
        assert_eq!(contact.get_name(), "bob1");
        assert_eq!(contact.get_display_name(), "bob1");

        // incoming mail `From: bob2 <bob@example.org>` - this should update authname and name
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "bob2",
            "bob@example.org",
            Origin::IncomingUnknownFrom,
        )
        .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "bob2");
        assert_eq!(contact.get_name(), "bob2");
        assert_eq!(contact.get_display_name(), "bob2");

        // manually edit name to "bob3" - authname should be still be "bob2" a given in `From:` above
        let contact_id = Contact::create(&t.ctx, "bob3", "bob@example.org").unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "bob2");
        assert_eq!(contact.get_name(), "bob3");
        assert_eq!(contact.get_display_name(), "bob3");

        // incoming mail `From: bob4 <bob@example.org>` - this should update authname, manually given name is still "bob3"
        let (contact_id, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "bob4",
            "bob@example.org",
            Origin::IncomingUnknownFrom,
        )
        .unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "bob4");
        assert_eq!(contact.get_name(), "bob3");
        assert_eq!(contact.get_display_name(), "bob3");
    }

    #[test]
    fn test_remote_authnames_create_empty() {
        let t = dummy_context();

        // manually create "claire@example.org" without a given name
        let contact_id = Contact::create(&t.ctx, "", "claire@example.org").unwrap();
        assert!(contact_id > DC_CONTACT_ID_LAST_SPECIAL);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "");
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "claire@example.org");

        // incoming mail `From: claire1 <claire@example.org>` - this should update authname and name
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "claire1",
            "claire@example.org",
            Origin::IncomingUnknownFrom,
        )
        .unwrap();
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "claire1");
        assert_eq!(contact.get_name(), "claire1");
        assert_eq!(contact.get_display_name(), "claire1");

        // incoming mail `From: claire2 <claire@example.org>` - this should update authname and name
        let (contact_id_same, sth_modified) = Contact::add_or_lookup(
            &t.ctx,
            "claire2",
            "claire@example.org",
            Origin::IncomingUnknownFrom,
        )
        .unwrap();
        assert_eq!(contact_id, contact_id_same);
        assert_eq!(sth_modified, Modifier::Modified);
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "claire2");
        assert_eq!(contact.get_name(), "claire2");
        assert_eq!(contact.get_display_name(), "claire2");
    }

    #[test]
    fn test_remote_authnames_edit_empty() {
        let t = dummy_context();

        // manually create "dave@example.org"
        let contact_id = Contact::create(&t.ctx, "dave1", "dave@example.org").unwrap();
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "");
        assert_eq!(contact.get_name(), "dave1");
        assert_eq!(contact.get_display_name(), "dave1");

        // incoming mail `From: dave2 <dave@example.org>` - this should update authname
        Contact::add_or_lookup(
            &t.ctx,
            "dave2",
            "dave@example.org",
            Origin::IncomingUnknownFrom,
        )
        .unwrap();
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "dave2");
        assert_eq!(contact.get_name(), "dave1");
        assert_eq!(contact.get_display_name(), "dave1");

        // manually clear the name
        Contact::create(&t.ctx, "", "dave@example.org").unwrap();
        let contact = Contact::load_from_db(&t.ctx, contact_id).unwrap();
        assert_eq!(contact.get_authname(), "dave2");
        assert_eq!(contact.get_name(), "dave2");
        assert_eq!(contact.get_display_name(), "dave2");
    }

    #[test]
    fn test_addr_cmp() {
        assert!(addr_cmp("AA@AA.ORG", "aa@aa.ORG"));
        assert!(addr_cmp(" aa@aa.ORG ", "AA@AA.ORG"));
        assert!(addr_cmp(" mailto:AA@AA.ORG", "Aa@Aa.orG"));
    }
}
