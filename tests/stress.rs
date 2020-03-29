//! Stress some functions for testing; if used as a lib, this file is obsolete.

use deltachat::config;
use deltachat::context::*;
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
    // let setupcode_c = CString::new(setupcode.clone()).unwrap();
    // let setupfile = dc_render_setup_file(context, &setupcode).unwrap();
    // let setupfile_c = CString::new(setupfile).unwrap();
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
