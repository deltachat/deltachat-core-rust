//! End-to-end encryption support.

use std::collections::HashSet;
use std::ptr;
use std::str::FromStr;

use libc::strlen;
use mmime::clist::*;
use mmime::mailimf::types::*;
use mmime::mailimf::types_helper::*;
use mmime::mailimf::*;
use mmime::mailmime::content::*;
use mmime::mailmime::types::*;
use mmime::mailmime::types_helper::*;
use mmime::mailmime::write_mem::*;
use mmime::mailmime::*;
use mmime::mailprivacy_prepare_mime;
use mmime::mmapstring::*;
use mmime::{mailmime_substitute, MAILIMF_NO_ERROR, MAIL_NO_ERROR};
use num_traits::FromPrimitive;

use crate::aheader::*;
use crate::config::Config;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::*;
use crate::key::*;
use crate::keyring::*;
use crate::mimefactory::MimeFactory;
use crate::peerstate::*;
use crate::pgp::*;
use crate::securejoin::handle_degrade_event;
use crate::wrapmime;
use crate::wrapmime::*;

// standard mime-version header aka b"Version: 1\r\n\x00"
static mut VERSION_CONTENT: [libc::c_char; 13] =
    [86, 101, 114, 115, 105, 111, 110, 58, 32, 49, 13, 10, 0];

type SignFingerprint = String;
type GossipAddr = String;

#[derive(Debug)]
pub struct EncryptHelper {
    pub prefer_encrypt: EncryptPreference,
    pub addr: String,
    pub public_key: Key,
}

impl EncryptHelper {
    pub fn new(context: &Context) -> Result<EncryptHelper> {
        let prefer_encrypt = context
            .sql
            .get_config_int(&context, "e2ee_enabled")
            .and_then(EncryptPreference::from_i32)
            .unwrap_or_default();

        let addr = match context.get_config(Config::ConfiguredAddr) {
            None => {
                bail!("addr not configured!");
            }
            Some(addr) => addr,
        };

        let public_key = load_or_generate_self_public_key(context, &addr)?;

        Ok(EncryptHelper {
            prefer_encrypt,
            addr,
            public_key,
        })
    }

    pub fn get_aheader(&self) -> Aheader {
        let pk = self.public_key.clone();
        let addr = self.addr.to_string();
        Aheader::new(addr, pk, self.prefer_encrypt)
    }

    pub fn try_encrypt(
        &mut self,
        factory: &mut MimeFactory,
        e2ee_guaranteed: bool,
        min_verified: libc::c_int,
        do_gossip: bool,
        mut in_out_message: *mut Mailmime,
        imffields_unprotected: *mut mailimf_fields,
    ) -> Result<bool> {
        // libEtPan's pgp_encrypt_mime() takes the parent as the new root.
        // We just expect the root as being given to this function.
        ensure!(
            !in_out_message.is_null() && unsafe { (*in_out_message).mm_parent.is_null() },
            "corrupted inputs"
        );

        if !(self.prefer_encrypt == EncryptPreference::Mutual || e2ee_guaranteed) {
            return Ok(false);
        }

        let context = &factory.context;
        let mut keyring = Keyring::default();
        let mut gossip_headers: Vec<String> = Vec::with_capacity(factory.recipients_addr.len());

        // determine if we can and should encrypt
        for recipient_addr in factory.recipients_addr.iter() {
            if recipient_addr == &self.addr {
                continue;
            }
            let peerstate = match Peerstate::from_addr(context, &context.sql, recipient_addr) {
                Some(peerstate) => peerstate,
                None => {
                    let msg = format!("peerstate for {} missing, cannot encrypt", recipient_addr);
                    if e2ee_guaranteed {
                        return Err(format_err!("{}", msg));
                    } else {
                        info!(context, "{}", msg);
                        return Ok(false);
                    }
                }
            };

            if peerstate.prefer_encrypt != EncryptPreference::Mutual && !e2ee_guaranteed {
                info!(context, "peerstate for {} is no-encrypt", recipient_addr);
                return Ok(false);
            }

            if let Some(key) = peerstate.peek_key(min_verified as usize) {
                keyring.add_owned(key.clone());
                if do_gossip {
                    if let Some(header) = peerstate.render_gossip_header(min_verified as usize) {
                        gossip_headers.push(header.to_string());
                    }
                }
            } else {
                bail!(
                    "proper enc-key for {} missing, cannot encrypt",
                    recipient_addr
                );
            }
        }

        let sign_key = {
            keyring.add_ref(&self.public_key);
            let key = Key::from_self_private(context, self.addr.clone(), &context.sql);
            ensure!(key.is_some(), "no own private key found");

            key
        };

        // encrypt message
        unsafe {
            mailprivacy_prepare_mime(in_out_message);
            let mut part_to_encrypt = (*in_out_message).mm_data.mm_message.mm_msg_mime;
            (*part_to_encrypt).mm_parent = ptr::null_mut();
            let imffields_encrypted = mailimf_fields_new_empty();

            // mailmime_new_message_data() calls mailmime_fields_new_with_version()
            // which would add the unwanted MIME-Version:-header
            let message_to_encrypt = mailmime_new_simple(
                MAILMIME_MESSAGE as libc::c_int,
                mailmime_fields_new_empty(),
                mailmime_get_content_message(),
                imffields_encrypted,
                part_to_encrypt,
            );

            for header in &gossip_headers {
                wrapmime::new_custom_field(imffields_encrypted, "Autocrypt-Gossip", &header)
            }

            // memoryhole headers: move some headers into encrypted part
            // XXX note we can't use clist's into_iter() because the loop body also removes items
            let mut cur = (*(*imffields_unprotected).fld_list).first;
            while !cur.is_null() {
                let field = (*cur).data as *mut mailimf_field;
                let mut move_to_encrypted = false;

                if !field.is_null() {
                    if (*field).fld_type == MAILIMF_FIELD_SUBJECT as libc::c_int {
                        move_to_encrypted = true;
                    } else if (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
                        let opt_field = (*field).fld_data.fld_optional_field;
                        if !opt_field.is_null() && !(*opt_field).fld_name.is_null() {
                            let fld_name = to_string_lossy((*opt_field).fld_name);
                            if fld_name.starts_with("Secure-Join")
                                || (fld_name.starts_with("Chat-") && fld_name != "Chat-Version")
                            {
                                move_to_encrypted = true;
                            }
                        }
                    }
                }

                if move_to_encrypted {
                    mailimf_fields_add(imffields_encrypted, field);
                    cur = clist_delete((*imffields_unprotected).fld_list, cur);
                } else {
                    cur = (*cur).next;
                }
            }

            let subject = mailimf_subject_new("...".strdup());
            mailimf_fields_add(imffields_unprotected, mailimf_field_new_subject(subject));

            wrapmime::append_ct_param(
                (*part_to_encrypt).mm_content_type,
                "protected-headers",
                "v1",
            )?;
            let plain = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
            let mut col = 0;
            mailmime_write_mem(plain, &mut col, message_to_encrypt);
            mailmime_free(message_to_encrypt);

            ensure!(
                !(*plain).str_0.is_null() && (*plain).len > 0,
                "could not write/allocate"
            );

            let ctext = dc_pgp_pk_encrypt(
                std::slice::from_raw_parts((*plain).str_0 as *const u8, (*plain).len),
                &keyring,
                sign_key.as_ref(),
            );
            mmap_string_free(plain);

            let ctext_v = ctext?;

            // create MIME-structure that will contain the encrypted text
            let mut encrypted_part = new_data_part(
                ptr::null_mut(),
                0 as libc::size_t,
                "multipart/encrypted",
                MAILMIME_MECHANISM_BASE64,
            )?;
            let content = (*encrypted_part).mm_content_type;
            wrapmime::append_ct_param(content, "protocol", "application/pgp-encrypted")?;

            let version_mime = new_data_part(
                VERSION_CONTENT.as_mut_ptr() as *mut libc::c_void,
                strlen(VERSION_CONTENT.as_mut_ptr()),
                "application/pgp-encrypted",
                MAILMIME_MECHANISM_7BIT,
            )?;
            mailmime_smart_add_part(encrypted_part, version_mime);

            // we assume that ctext_v is not dropped until the end
            // of this if-scope
            let ctext_part = new_data_part(
                ctext_v.as_ptr() as *mut libc::c_void,
                ctext_v.len(),
                "application/octet-stream",
                MAILMIME_MECHANISM_7BIT,
            )?;

            mailmime_smart_add_part(encrypted_part, ctext_part);
            (*in_out_message).mm_data.mm_message.mm_msg_mime = encrypted_part;
            (*encrypted_part).mm_parent = in_out_message;
            let gossiped = !&gossip_headers.is_empty();
            factory.finalize_mime_message(in_out_message, true, gossiped)?;

            Ok(true)
        }
    }
}

pub fn try_decrypt(
    context: &Context,
    in_out_message: *mut Mailmime,
) -> Result<DecryptInfo> {
    let imffields = unsafe { mailmime_find_mailimf_fields(in_out_message) };
    ensure!(
        !in_out_message.is_null() && !imffields.is_null(),
        "corrupt invalid mime inputs"
    );

    let from = wrapmime::get_field_from(imffields)?;
    let message_time = wrapmime::get_field_date(imffields)?;

    let autocryptheader = Aheader::from_imffields(&from, imffields);
    let recipients = mailimf_get_recipients(imffields);
    let mut peerstate = None;

    if message_time > 0 {
        peerstate = Peerstate::from_addr(context, &context.sql, &from);

        if let Some(ref mut peerstate) = peerstate {
            // update Autocrypt peer state as per 
            // https://autocrypt.org/level1.html#updating-autocrypt-peer-state
            if let Some(ref header) = autocryptheader {
                peerstate.apply_header(&header, message_time);
                peerstate.save_to_db(&context.sql, false).unwrap();
            } else if message_time > peerstate.last_seen_autocrypt
                && !contains_report(in_out_message)
            {
                peerstate.degrade_encryption(message_time);
                peerstate.save_to_db(&context.sql, false).unwrap();
            }
        } else if let Some(ref header) = autocryptheader {
            let p = Peerstate::from_header(context, header, message_time);
            p.save_to_db(&context.sql, true).unwrap();
            peerstate = Some(p);
        }
    }
    /* possibly perform decryption */
    let mut private_keyring = Keyring::default();
    let mut public_keyring_for_validate = Keyring::default();
    let mut signatures = HashSet::default();
    let mut gossipped_addr = HashSet::default();

    let self_addr = context.get_config(Config::ConfiguredAddr);

    if let Some(self_addr) = self_addr {
        if private_keyring.load_self_private_for_decrypting(context, self_addr, &context.sql) {
            if peerstate.as_ref().map(|p| p.last_seen).unwrap_or_else(|| 0) == 0 {
                peerstate = Peerstate::from_addr(&context, &context.sql, &from);
            }
            if let Some(ref peerstate) = peerstate {
                if peerstate.degrade_event.is_some() {
                    handle_degrade_event(context, &peerstate)?;
                }
                if let Some(ref key) = peerstate.gossip_key {
                    public_keyring_for_validate.add_ref(key);
                }
                if let Some(ref key) = peerstate.public_key {
                    public_keyring_for_validate.add_ref(key);
                }
            }

            let mut gossip_headers = ptr::null_mut();
            let mut decrypt_info = decrypt_if_autocrypt_message(
                context,
                in_out_message,
                &private_keyring,
                &public_keyring_for_validate,
            )?;
            decrypt_info.gossipped_addrs =
                update_gossip_peerstates(
                    context, 
                    message_time, 
                    gosimffields, decrypt_info.gossip_headers)?;
            if recipients.is_none() {
                recipients = Some(
            }
            }
        }
    }
    Ok((!signatures.is_empty(), signatures, gossipped_addr))
}

fn new_data_part(
    data: *mut libc::c_void,
    data_bytes: libc::size_t,
    content_type: &str,
    default_encoding: u32,
) -> Result<*mut Mailmime> {
    let content = new_content_type(&content_type)?;
    let mut encoding = ptr::null_mut();
    if wrapmime::content_type_needs_encoding(content) {
        encoding = unsafe { mailmime_mechanism_new(default_encoding as i32, ptr::null_mut()) };
        ensure!(!encoding.is_null(), "failed to create encoding");
    }
    let mime_fields = {
        unsafe {
            mailmime_fields_new_with_data(
                encoding,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        }
    };
    ensure!(!mime_fields.is_null(), "internal mime error");

    let mime = unsafe { mailmime_new_empty(content, mime_fields) };
    ensure!(!mime.is_null(), "internal mime error");

    if unsafe { (*mime).mm_type } == MAILMIME_SINGLE as libc::c_int {
        if !data.is_null() && data_bytes > 0 {
            unsafe { mailmime_set_body_text(mime, data as *mut libc::c_char, data_bytes) };
        }
    }

    Ok(mime)
}

/// Load public key from database or generate a new one.
///
/// This will load a public key from the database, generating and
/// storing a new one when one doesn't exist yet.  Care is taken to
/// only generate one key per context even when multiple threads call
/// this function concurrently.
fn load_or_generate_self_public_key(context: &Context, self_addr: impl AsRef<str>) -> Result<Key> {
    if let Some(key) = Key::from_self_public(context, &self_addr, &context.sql) {
        return Ok(key);
    }
    let _guard = context.generating_key_mutex.lock().unwrap();

    // Check again in case the key was generated while we were waiting for the lock.
    if let Some(key) = Key::from_self_public(context, &self_addr, &context.sql) {
        return Ok(key);
    }

    let start = std::time::Instant::now();
    info!(
        context,
        "Generating keypair with {} bits, e={} ...", 2048, 65537,
    );
    match dc_pgp_create_keypair(&self_addr) {
        Some((public_key, private_key)) => {
            match dc_key_save_self_keypair(
                context,
                &public_key,
                &private_key,
                &self_addr,
                true,
                &context.sql,
            ) {
                true => {
                    info!(
                        context,
                        "Keypair generated in {:.3}s.",
                        start.elapsed().as_secs()
                    );
                    Ok(public_key)
                }
                false => Err(format_err!("Failed to save keypair")),
            }
        }
        None => Err(format_err!("Failed to generate keypair")),
    }
}

fn update_gossip_peerstates(
    context: &Context,
    message_time: i64,
    recipients: &Vec<String>,
    gossip_header_values: &Vec<String>,
) -> Result<HashSet<String>> {
    // XXX split the parsing from the modification part
    let recipients = wrapmime::mailimf_get_recipients(imffields);

    let mut gossipped_addr = HashSet<String>::default();
    for value in gossip_header_values.iter() {
        let gossip_header = Aheader::from_str(&value);

        if let Ok(ref header) = gossip_header {
            if recipients.as_ref().unwrap().contains(&header.addr) {
                let mut peerstate = Peerstate::from_addr(context, &context.sql, &header.addr);
                if let Some(ref mut peerstate) = peerstate {
                    peerstate.apply_gossip(header, message_time);
                    peerstate.save_to_db(&context.sql, false)?;
                } else {
                    let p = Peerstate::from_gossip(context, header, message_time);
                    p.save_to_db(&context.sql, true)?;
                    peerstate = Some(p);
                }
                if let Some(peerstate) = peerstate {
                    if peerstate.degrade_event.is_some() {
                        handle_degrade_event(context, &peerstate)?;
                    }
                }

                gossipped_addr.insert(header.addr.clone());
            } else {
                info!(
                    context,
                    "Ignoring gossipped \"{}\" as the address is not in To/Cc list.", &header.addr,
                );
            }
        }
    }

    Ok(gossipped_addr)
}

#[derive(Debug,Default)]
struct DecryptInfo {
    signers: Vec<String>;
    gossiped_addrs: HashSet<String>;
    was_encrypted: bool;
}

fn decrypt_if_autocrypt_message(
    context: &Context,
    mime_undetermined: *mut Mailmime,
    private_keyring: &Keyring,
    public_keyring_for_validate: &Keyring,
) -> Result<DecryptInfo> {
    /* If we detected an Autocrypt-encrypted message and successfully decrypted it
    there will be at least one signer in the returned Vec<Signer>. 

    Decryption modifies the passed in mime structure in place.  It is possible
    to get a decrypted message yet it is not signed.  This should be interpreted
    as not-encrypted (i.e. no lock in UIs) on messages. 

    Errors are returned for failures related to decryption of AC-messages.
    */
    ensure!(!mime_undetermined.is_null(), "Invalid mime reference");

    let (mime, encrypted_data_part) = match wrapmime::get_autocrypt_mime(mime_undetermined) {
        Err(_) => {
            // not a proper autocrypt message, abort and ignore
            return Ok(DecryptInfo::default());
        }
        Ok(res) => res,
    };

    decrypt_part(
        encrypted_data_part,
        private_keyring,
        public_keyring_for_validate,
    )
}


fn decrypt_part(
    mime: *mut Mailmime,
    private_keyring: &Keyring,
    public_keyring_for_validate: &Keyring,
) -> Result<DecryptInfo> {
    let mime_data: *mut mailmime_data;

    unsafe {
        mime_data = (*mime).mm_data.mm_single;
    }
    if !wrapmime::has_decryptable_data(mime_data) {
        return Ok(DecryptInfo::default())
    }

    let mut mime_transfer_encoding = MAILMIME_MECHANISM_BINARY as libc::c_int;
    if let Some(enc) = wrapmime::get_mime_transfer_encoding(mime) {
        mime_transfer_encoding = enc;
    }

    let data: Vec<u8> = wrapmime::decode_dt_data(mime_data, mime_transfer_encoding)?;

    if !has_decrypted_pgp_armor(&data) {
        return Ok(DecryptInfo::default())
    }

    // now we have prepared ourselves to attempt decryption 

    let mut decrypt_info = DecryptInfo::default();
    let plain = match dc_pgp_pk_decrypt(
        &data,
        &private_keyring,
        &public_keyring_for_validate,
        Some(decrypt_info.signers),
    ) {
        Ok(plain) => {
            decrypt_info.plain = plain;
            plain
        }
        Err(err) => bail!("could not decrypt: {}", err),
    };
    if let Some(decrypted_mime) = wrapmime::parse_mailmime(data) {
        unsafe {
            // mailmime_substitute detaches mime from its position in the mime tree 
            // and puts decrypted_mime in its position. Afterwars mime is dangling. 
            mailmime_substitute(mime, decrypted_mime);
            mailmime_free(mime);
        }

        // now let's read all gossip-header values 
        decrypt_info.gossip_addrs = wrapmime::iter_optional_field_values(
            decrypted_mime,
            b"Autocrypt-Gossip\0" as *const u8 as *const libc::c_char,
        )?;
    } else {
        decrypt_info.parsing_error_after_decryption = true;
    }
    Ok(decrypt_info)
}

fn has_decrypted_pgp_armor(input: &[u8]) -> bool {
    if let Some(index) = input.iter().position(|b| *b > b' ') {
        if input.len() - index > 26 {
            let start = index;
            let end = start + 27;

            return &input[start..end] == b"-----BEGIN PGP MESSAGE-----";
        }
    }

    false
}

/// Check if a MIME structure contains a multipart/report part.
///
/// As reports are often unencrypted, we do not reset the Autocrypt header in
/// this case.
///
/// However, Delta Chat itself has no problem with encrypted multipart/report
/// parts and MUAs should be encouraged to encrpyt multipart/reports as well so
/// that we could use the normal Autocrypt processing.
fn contains_report(mime: *mut Mailmime) -> bool {
    assert!(!mime.is_null());
    let mime = unsafe { *mime };

    if mime.mm_type == MAILMIME_MULTIPLE as libc::c_int {
        let tp_type = unsafe { (*(*mime.mm_content_type).ct_type).tp_type };
        let ct_type =
            unsafe { (*(*(*mime.mm_content_type).ct_type).tp_data.tp_composite_type).ct_type };

        if tp_type == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int
            && ct_type == MAILMIME_COMPOSITE_TYPE_MULTIPART as libc::c_int
            && as_str(unsafe { (*mime.mm_content_type).ct_subtype }) == "report"
        {
            return true;
        }

        for cur_data in unsafe { (*(*mime.mm_mime_fields).fld_list).into_iter() } {
            if contains_report(cur_data as *mut Mailmime) {
                return true;
            }
        }
    } else if mime.mm_type == MAILMIME_MESSAGE as libc::c_int {
        let m = unsafe { mime.mm_data.mm_message.mm_msg_mime };

        if contains_report(m) {
            return true;
        }
    }

    false
}

/// Ensures a private key exists for the configured user.
///
/// Normally the private key is generated when the first message is
/// sent but in a few locations there are no such guarantees,
/// e.g. when exporting keys, and calling this function ensures a
/// private key will be present.
///
/// If this succeeds you are also guaranteed that the
/// [Config::ConfiguredAddr] is configured, this address is returned.
pub fn ensure_secret_key_exists(context: &Context) -> Result<String> {
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .ok_or(format_err!(concat!(
            "Failed to get self address, ",
            "cannot ensure secret key if not configured."
        )))?;
    load_or_generate_self_public_key(context, &self_addr)?;
    Ok(self_addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::free;

    use crate::test_utils::*;

    mod ensure_secret_key_exists {
        use super::*;

        #[test]
        fn test_prexisting() {
            let t = dummy_context();
            let test_addr = configure_alice_keypair(&t.ctx);
            assert_eq!(ensure_secret_key_exists(&t.ctx).unwrap(), test_addr);
        }

        #[test]
        fn test_not_configured() {
            let t = dummy_context();
            assert!(ensure_secret_key_exists(&t.ctx).is_err());
        }
    }

    #[test]
    fn test_mailmime_parse() {
        let plain = b"Chat-Disposition-Notification-To: holger@deltachat.de
Chat-Group-ID: CovhGgau8M-
Chat-Group-Name: Delta Chat Dev
Subject: =?utf-8?Q?Chat=3A?= Delta Chat =?utf-8?Q?Dev=3A?= sidenote for
 =?utf-8?Q?all=3A?= rust core master ...
Content-Type: text/plain; charset=\"utf-8\"; protected-headers=\"v1\"
Content-Transfer-Encoding: quoted-printable

sidenote for all: rust core master is broken currently ... so dont recomm=
end to try to run with desktop or ios unless you are ready to hunt bugs

-- =20
Sent with my Delta Chat Messenger: https://delta.chat";
        let plain_bytes = plain.len();
        let plain_buf = plain.as_ptr() as *const libc::c_char;

        let mut index = 0;
        let mut decrypted_mime = std::ptr::null_mut();

        let res = unsafe {
            mailmime_parse(
                plain_buf as *const _,
                plain_bytes,
                &mut index,
                &mut decrypted_mime,
            )
        };
        unsafe {
            let msg1 = (*decrypted_mime).mm_data.mm_message.mm_msg_mime;
            let data = mailmime_transfer_decode(msg1).unwrap();
            println!("{:?}", String::from_utf8_lossy(&data));
        }

        assert_eq!(res, 0);
        assert!(!decrypted_mime.is_null());

        unsafe { free(decrypted_mime as *mut _) };
    }

    mod load_or_generate_self_public_key {
        use super::*;

        #[test]
        fn test_existing() {
            let t = dummy_context();
            let addr = configure_alice_keypair(&t.ctx);
            let key = load_or_generate_self_public_key(&t.ctx, addr);
            assert!(key.is_ok());
        }

        #[test]
        #[ignore] // generating keys is expensive
        fn test_generate() {
            let t = dummy_context();
            let addr = "alice@example.org";
            let key0 = load_or_generate_self_public_key(&t.ctx, addr);
            assert!(key0.is_ok());
            let key1 = load_or_generate_self_public_key(&t.ctx, addr);
            assert!(key1.is_ok());
            assert_eq!(key0.unwrap(), key1.unwrap());
        }

        #[test]
        #[ignore]
        fn test_generate_concurrent() {
            use std::sync::Arc;
            use std::thread;

            let t = dummy_context();
            let ctx = Arc::new(t.ctx);
            let ctx0 = Arc::clone(&ctx);
            let thr0 =
                thread::spawn(move || load_or_generate_self_public_key(&ctx0, "alice@example.org"));
            let ctx1 = Arc::clone(&ctx);
            let thr1 =
                thread::spawn(move || load_or_generate_self_public_key(&ctx1, "alice@example.org"));
            let res0 = thr0.join().unwrap();
            let res1 = thr1.join().unwrap();
            assert_eq!(res0.unwrap(), res1.unwrap());
        }
    }

    #[test]
    fn test_has_decrypted_pgp_armor() {
        let data = b" -----BEGIN PGP MESSAGE-----";
        assert_eq!(has_decrypted_pgp_armor(data), true);

        let data = b"    \n-----BEGIN PGP MESSAGE-----";
        assert_eq!(has_decrypted_pgp_armor(data), true);

        let data = b"    -----BEGIN PGP MESSAGE---";
        assert_eq!(has_decrypted_pgp_armor(data), false);

        let data = b" -----BEGIN PGP MESSAGE-----";
        assert_eq!(has_decrypted_pgp_armor(data), true);

        let data = b"blas";
        assert_eq!(has_decrypted_pgp_armor(data), false);
    }
}
