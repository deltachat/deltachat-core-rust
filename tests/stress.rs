//! Stress some functions for testing; if used as a lib, this file is obsolete.

use std::collections::HashSet;

use deltachat::config;
use deltachat::context::*;
use deltachat::keyring::*;
use deltachat::pgp;
use deltachat::Event;
use tempfile::{tempdir, TempDir};

/* some data used for testing
 ******************************************************************************/

fn stress_functions(context: &Context) {
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
    let (public_key, private_key) = pgp::create_keypair("foo@bar.de").unwrap();

    private_key.split_key().unwrap();

    let (public_key2, private_key2) = pgp::create_keypair("two@zwo.de").unwrap();

    assert_ne!(public_key, public_key2);

    let original_text = b"This is a test";
    let mut keyring = Keyring::default();
    keyring.add_owned(public_key.clone());
    keyring.add_ref(&public_key2);

    let ctext_signed = pgp::pk_encrypt(original_text, &keyring, Some(&private_key)).unwrap();
    assert!(!ctext_signed.is_empty());
    assert!(ctext_signed.starts_with("-----BEGIN PGP MESSAGE-----"));

    let ctext_unsigned = pgp::pk_encrypt(original_text, &keyring, None).unwrap();
    assert!(!ctext_unsigned.is_empty());
    assert!(ctext_unsigned.starts_with("-----BEGIN PGP MESSAGE-----"));

    let mut keyring = Keyring::default();
    keyring.add_owned(private_key);

    let mut public_keyring = Keyring::default();
    public_keyring.add_ref(&public_key);

    let mut public_keyring2 = Keyring::default();
    public_keyring2.add_owned(public_key2);

    let mut valid_signatures: HashSet<String> = Default::default();

    let plain = pgp::pk_decrypt(
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
    let plain = pgp::pk_decrypt(
        ctext_signed.as_bytes(),
        &keyring,
        &empty_keyring,
        Some(&mut valid_signatures),
    )
    .unwrap();
    assert_eq!(plain, original_text);
    assert_eq!(valid_signatures.len(), 0);

    valid_signatures.clear();

    let plain = pgp::pk_decrypt(
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

    let plain = pgp::pk_decrypt(
        ctext_signed.as_bytes(),
        &keyring,
        &public_keyring2,
        Some(&mut valid_signatures),
    )
    .unwrap();
    assert_eq!(plain, original_text);
    assert_eq!(valid_signatures.len(), 1);

    valid_signatures.clear();

    let plain = pgp::pk_decrypt(
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

    let plain = pgp::pk_decrypt(ctext_signed.as_bytes(), &keyring, &public_keyring, None).unwrap();

    assert_eq!(plain, original_text);
}

fn cb(_context: &Context, _event: Event) {}

#[allow(dead_code)]
struct TestContext {
    ctx: Context,
    dir: TempDir,
}

fn create_test_context() -> TestContext {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let ctx = Context::new(Box::new(cb), "FakeOs".into(), dbfile).unwrap();
    TestContext { ctx, dir }
}

#[test]
fn test_stress_tests() {
    let context = create_test_context();
    stress_functions(&context.ctx);
}
