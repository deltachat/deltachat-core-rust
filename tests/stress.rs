//! Stress some functions for testing; if used as a lib, this file is obsolete.

use std::collections::HashSet;
use std::ptr;

use deltachat::chat::{self, Chat};
use deltachat::config;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::dc_tools::*;
use deltachat::keyring::*;
use deltachat::oauth2::*;
use deltachat::pgp::*;
use deltachat::Event;
use libc::{free, strcmp, strdup};
use tempfile::{tempdir, TempDir};

/* some data used for testing
 ******************************************************************************/

unsafe fn stress_functions(context: &Context) {
    let res = context.get_config(config::Config::SysConfigKeys).unwrap();

    assert!(!res.contains(" probably_never_a_key "));
    assert!(res.contains(" addr "));
    assert!(res.contains(" mail_server "));
    assert!(res.contains(" mail_user "));
    assert!(res.contains(" mail_pw "));
    assert!(res.contains(" mail_port "));
    assert!(res.contains(" send_server "));
    assert!(res.contains(" send_user "));
    assert!(res.contains(" send_pw "));
    assert!(res.contains(" send_port "));
    assert!(res.contains(" server_flags "));
    assert!(res.contains(" imap_folder "));
    assert!(res.contains(" displayname "));
    assert!(res.contains(" selfstatus "));
    assert!(res.contains(" selfavatar "));
    assert!(res.contains(" e2ee_enabled "));
    assert!(res.contains(" mdns_enabled "));
    assert!(res.contains(" save_mime_headers "));
    assert!(res.contains(" configured_addr "));
    assert!(res.contains(" configured_mail_server "));
    assert!(res.contains(" configured_mail_user "));
    assert!(res.contains(" configured_mail_pw "));
    assert!(res.contains(" configured_mail_port "));
    assert!(res.contains(" configured_send_server "));
    assert!(res.contains(" configured_send_user "));
    assert!(res.contains(" configured_send_pw "));
    assert!(res.contains(" configured_send_port "));
    assert!(res.contains(" configured_server_flags "));

    let mut buf_0: *mut libc::c_char;
    let mut headerline = String::default();
    let mut setupcodebegin: *const libc::c_char = ptr::null();
    let mut preferencrypt: *const libc::c_char = ptr::null();
    let mut base64: *const libc::c_char = ptr::null();
    buf_0 = strdup(
        b"-----BEGIN PGP MESSAGE-----\nNoVal:\n\ndata\n-----END PGP MESSAGE-----\x00" as *const u8
            as *const libc::c_char,
    );
    let ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        ptr::null_mut(),
        &mut base64,
    );
    assert!(ok);
    assert!(!headerline.is_empty());
    assert_eq!(headerline, "-----BEGIN PGP MESSAGE-----");

    assert!(!base64.is_null());
    assert_eq!(as_str(base64 as *const libc::c_char), "data",);

    free(buf_0 as *mut libc::c_void);

    buf_0 =
        strdup(b"-----BEGIN PGP MESSAGE-----\n\ndat1\n-----END PGP MESSAGE-----\n-----BEGIN PGP MESSAGE-----\n\ndat2\n-----END PGP MESSAGE-----\x00"
                   as *const u8 as *const libc::c_char);
    let ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        ptr::null_mut(),
        &mut base64,
    );

    assert!(ok);
    assert_eq!(headerline, "-----BEGIN PGP MESSAGE-----");

    assert!(!base64.is_null());
    assert_eq!(as_str(base64 as *const libc::c_char), "dat1",);

    free(buf_0 as *mut libc::c_void);

    buf_0 = strdup(
        b"foo \n -----BEGIN PGP MESSAGE----- \n base64-123 \n  -----END PGP MESSAGE-----\x00"
            as *const u8 as *const libc::c_char,
    );
    let ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        ptr::null_mut(),
        &mut base64,
    );

    assert!(ok);
    assert_eq!(headerline, "-----BEGIN PGP MESSAGE-----");
    assert!(setupcodebegin.is_null());

    assert!(!base64.is_null());
    assert_eq!(as_str(base64 as *const libc::c_char), "base64-123",);

    free(buf_0 as *mut libc::c_void);

    buf_0 = strdup(b"foo-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char);
    let ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        ptr::null_mut(),
        &mut base64,
    );

    assert!(!ok);
    free(buf_0 as *mut libc::c_void);
    buf_0 =
        strdup(b"foo \n -----BEGIN PGP MESSAGE-----\n  Passphrase-BeGIN  :  23 \n  \n base64-567 \r\n abc \n  -----END PGP MESSAGE-----\n\n\n\x00"
                   as *const u8 as *const libc::c_char);
    let ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        ptr::null_mut(),
        &mut base64,
    );
    assert!(ok);
    assert_eq!(headerline, "-----BEGIN PGP MESSAGE-----");

    assert!(!setupcodebegin.is_null());
    assert_eq!(
        strcmp(
            setupcodebegin,
            b"23\x00" as *const u8 as *const libc::c_char,
        ),
        0
    );

    assert!(!base64.is_null());
    assert_eq!(as_str(base64 as *const libc::c_char), "base64-567 \n abc",);

    free(buf_0 as *mut libc::c_void);

    buf_0 =
        strdup(b"-----BEGIN PGP PRIVATE KEY BLOCK-----\n Autocrypt-Prefer-Encrypt :  mutual \n\nbase64\n-----END PGP PRIVATE KEY BLOCK-----\x00"
                   as *const u8 as *const libc::c_char);
    let ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        ptr::null_mut(),
        &mut preferencrypt,
        &mut base64,
    );
    assert!(ok);
    assert_eq!(headerline, "-----BEGIN PGP PRIVATE KEY BLOCK-----");
    assert!(!preferencrypt.is_null());
    assert_eq!(
        strcmp(
            preferencrypt,
            b"mutual\x00" as *const u8 as *const libc::c_char,
        ),
        0
    );

    assert!(!base64.is_null());
    assert_eq!(as_str(base64 as *const libc::c_char), "base64",);

    free(buf_0 as *mut libc::c_void);

    // Cant check, no configured context
    // assert!(dc_is_configured(context) != 0, "Missing configured context");

    // let setupcode = dc_create_setup_code(context);
    // let setupcode_c = CString::yolo(setupcode.clone());
    // let setupfile = dc_render_setup_file(context, &setupcode).unwrap();
    // let setupfile_c = CString::yolo(setupfile);
    // let mut headerline_2: *const libc::c_char = ptr::null();
    // let payload = dc_decrypt_setup_file(context, setupcode_c.as_ptr(), setupfile_c.as_ptr());

    // assert!(payload.is_null());
    // assert!(!dc_split_armored_data(
    //     payload,
    //     &mut headerline_2,
    //     ptr::null_mut(),
    //     ptr::null_mut(),
    //     ptr::null_mut(),
    // ));
    // assert!(!headerline_2.is_null());
    // assert_eq!(
    //     strcmp(
    //         headerline_2,
    //         b"-----BEGIN PGP PRIVATE KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
    //     ),
    //     0
    // );
    // free(payload as *mut libc::c_void);

    // Cant check, no configured context
    // assert!(dc_is_configured(context) != 0, "missing configured context");

    // let qr = dc_get_securejoin_qr(context, 0);
    // assert!(!qr.is_null(), "Invalid qr code generated");
    // let qr_r = as_str(qr);

    // assert!(qr_r.len() > 55);
    // assert!(qr_r.starts_with("OPENPGP4FPR:"));

    // let res = dc_check_qr(context, qr);
    // let s = res.get_state();

    // assert!(
    //     s == QrState::AskVerifyContact
    //         || s == QrState::FprMissmatch
    //         || s == QrState::FprWithoutAddr
    // );

    // free(qr.cast());
}

#[test]
#[ignore] // is too expensive
fn test_encryption_decryption() {
    let (public_key, private_key) = dc_pgp_create_keypair("foo@bar.de").unwrap();

    private_key.split_key().unwrap();

    let (public_key2, private_key2) = dc_pgp_create_keypair("two@zwo.de").unwrap();

    assert_ne!(public_key, public_key2);

    let original_text = b"This is a test";
    let mut keyring = Keyring::default();
    keyring.add_owned(public_key.clone());
    keyring.add_ref(&public_key2);

    let ctext_signed = dc_pgp_pk_encrypt(original_text, &keyring, Some(&private_key)).unwrap();
    assert!(!ctext_signed.is_empty());
    assert!(ctext_signed.starts_with("-----BEGIN PGP MESSAGE-----"));

    let ctext_unsigned = dc_pgp_pk_encrypt(original_text, &keyring, None).unwrap();
    assert!(!ctext_unsigned.is_empty());
    assert!(ctext_unsigned.starts_with("-----BEGIN PGP MESSAGE-----"));

    let mut keyring = Keyring::default();
    keyring.add_owned(private_key);

    let mut public_keyring = Keyring::default();
    public_keyring.add_ref(&public_key);

    let mut public_keyring2 = Keyring::default();
    public_keyring2.add_owned(public_key2.clone());

    let mut valid_signatures: HashSet<String> = Default::default();

    let plain = dc_pgp_pk_decrypt(
        ctext_signed.as_bytes(),
        &keyring,
        &public_keyring,
        Some(&mut valid_signatures),
    )
    .unwrap();

    assert_eq!(plain, original_text,);
    assert_eq!(valid_signatures.len(), 1);

    valid_signatures.clear();

    let empty_keyring = Keyring::default();
    let plain = dc_pgp_pk_decrypt(
        ctext_signed.as_bytes(),
        &keyring,
        &empty_keyring,
        Some(&mut valid_signatures),
    )
    .unwrap();
    assert_eq!(plain, original_text);
    assert_eq!(valid_signatures.len(), 0);

    valid_signatures.clear();

    let plain = dc_pgp_pk_decrypt(
        ctext_signed.as_bytes(),
        &keyring,
        &public_keyring2,
        Some(&mut valid_signatures),
    )
    .unwrap();
    assert_eq!(plain, original_text);
    assert_eq!(valid_signatures.len(), 0);

    valid_signatures.clear();

    public_keyring2.add_ref(&public_key);

    let plain = dc_pgp_pk_decrypt(
        ctext_signed.as_bytes(),
        &keyring,
        &public_keyring2,
        Some(&mut valid_signatures),
    )
    .unwrap();
    assert_eq!(plain, original_text);
    assert_eq!(valid_signatures.len(), 1);

    valid_signatures.clear();

    let plain = dc_pgp_pk_decrypt(
        ctext_unsigned.as_bytes(),
        &keyring,
        &public_keyring,
        Some(&mut valid_signatures),
    )
    .unwrap();

    assert_eq!(plain, original_text);

    valid_signatures.clear();

    let mut keyring = Keyring::default();
    keyring.add_ref(&private_key2);
    let mut public_keyring = Keyring::default();
    public_keyring.add_ref(&public_key);

    let plain =
        dc_pgp_pk_decrypt(ctext_signed.as_bytes(), &keyring, &public_keyring, None).unwrap();

    assert_eq!(plain, original_text);
}

fn cb(_context: &Context, _event: Event) -> libc::uintptr_t {
    0
}

#[allow(dead_code)]
struct TestContext {
    ctx: Context,
    dir: TempDir,
}

fn create_test_context() -> TestContext {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let ctx = Context::new(Box::new(cb), "FakeOs".into(), dbfile).unwrap();
    TestContext { ctx: ctx, dir: dir }
}

#[test]
fn test_dc_get_oauth2_url() {
    let ctx = create_test_context();
    let addr = "dignifiedquire@gmail.com";
    let redirect_uri = "chat.delta:/com.b44t.messenger";
    let res = dc_get_oauth2_url(&ctx.ctx, addr, redirect_uri);

    assert_eq!(res, Some("https://accounts.google.com/o/oauth2/auth?client_id=959970109878%2D4mvtgf6feshskf7695nfln6002mom908%2Eapps%2Egoogleusercontent%2Ecom&redirect_uri=chat%2Edelta%3A%2Fcom%2Eb44t%2Emessenger&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F%20email&access_type=offline".into()));
}

#[test]
fn test_dc_get_oauth2_addr() {
    let ctx = create_test_context();
    let addr = "dignifiedquire@gmail.com";
    let code = "fail";
    let res = dc_get_oauth2_addr(&ctx.ctx, addr, code);
    // this should fail as it is an invalid password
    assert_eq!(res, None);
}

#[test]
fn test_dc_get_oauth2_token() {
    let ctx = create_test_context();
    let addr = "dignifiedquire@gmail.com";
    let code = "fail";
    let res = dc_get_oauth2_access_token(&ctx.ctx, addr, code, false);
    // this should fail as it is an invalid password
    assert_eq!(res, None);
}

#[test]
fn test_stress_tests() {
    unsafe {
        let context = create_test_context();
        stress_functions(&context.ctx);
    }
}

#[test]
fn test_get_contacts() {
    let context = create_test_context();
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
fn test_chat() {
    let context = create_test_context();
    let contact1 = Contact::create(&context.ctx, "bob", "bob@mail.de").unwrap();
    assert_ne!(contact1, 0);

    let chat_id = chat::create_by_contact_id(&context.ctx, contact1).unwrap();
    assert!(chat_id > 9, "chat_id too small {}", chat_id);
    let chat = Chat::load_from_db(&context.ctx, chat_id).unwrap();

    let chat2_id = chat::create_by_contact_id(&context.ctx, contact1).unwrap();
    assert_eq!(chat2_id, chat_id);
    let chat2 = Chat::load_from_db(&context.ctx, chat2_id).unwrap();

    assert_eq!(chat2.name, chat.name);
}
