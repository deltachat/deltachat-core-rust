//! Stress some functions for testing; if used as a lib, this file is obsolete.

use std::collections::HashSet;
use std::ffi::{CStr, CString};

use mmime::mailimf_types::*;
use tempfile::{tempdir, TempDir};

use deltachat::constants::*;
use deltachat::context::*;
use deltachat::dc_array::*;
use deltachat::dc_configure::*;
use deltachat::dc_imex::*;
use deltachat::dc_location::*;
use deltachat::dc_lot::*;
use deltachat::dc_mimeparser::*;
use deltachat::dc_param::*;
use deltachat::dc_qr::*;
use deltachat::dc_saxparser::*;
use deltachat::dc_securejoin::*;
use deltachat::dc_strbuilder::*;
use deltachat::dc_tools::*;
use deltachat::key::*;
use deltachat::keyring::*;
use deltachat::oauth2::*;
use deltachat::pgp::*;
use deltachat::types::*;
use deltachat::x::*;
use libc;

/* some data used for testing
 ******************************************************************************/
/* S_EM_SETUPFILE is a AES-256 symm. encrypted setup message created by Enigmail
with an "encrypted session key", see RFC 4880.  The code is in S_EM_SETUPCODE */
static mut S_EM_SETUPCODE: *const libc::c_char =
    b"1742-0185-6197-1303-7016-8412-3581-4441-0597\x00" as *const u8 as *const libc::c_char;
static mut S_EM_SETUPFILE: *const libc::c_char =
    b"-----BEGIN PGP MESSAGE-----\nPassphrase-Format: numeric9x4\nPassphrase-Begin: 17\n\n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\nmngm1QCMMw2v/j8=\n=9sJE\n-----END PGP MESSAGE-----\n\x00"
        as *const u8 as *const libc::c_char;

unsafe fn stress_functions(context: &Context) {
    let mut saxparser: dc_saxparser_t = dc_saxparser_t {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };
    dc_saxparser_init(&mut saxparser, 0 as *mut libc::c_void);
    dc_saxparser_parse(
        &mut saxparser,
        b"<tag attr=val=\x00" as *const u8 as *const libc::c_char,
    );
    dc_saxparser_parse(
        &mut saxparser,
        b"<tag attr=\"val\"=\x00" as *const u8 as *const libc::c_char,
    );

    if 0 != dc_is_open(context) {
        if 0 != dc_file_exist(
            context,
            b"$BLOBDIR/foobar\x00" as *const u8 as *const libc::c_char,
        ) || 0
            != dc_file_exist(
                context,
                b"$BLOBDIR/dada\x00" as *const u8 as *const libc::c_char,
            )
            || 0 != dc_file_exist(
                context,
                b"$BLOBDIR/foobar.dadada\x00" as *const u8 as *const libc::c_char,
            )
            || 0 != dc_file_exist(
                context,
                b"$BLOBDIR/foobar-folder\x00" as *const u8 as *const libc::c_char,
            )
        {
            dc_delete_file(
                context,
                b"$BLOBDIR/foobar\x00" as *const u8 as *const libc::c_char,
            );
            dc_delete_file(
                context,
                b"$BLOBDIR/dada\x00" as *const u8 as *const libc::c_char,
            );
            dc_delete_file(
                context,
                b"$BLOBDIR/foobar.dadada\x00" as *const u8 as *const libc::c_char,
            );
            dc_delete_file(
                context,
                b"$BLOBDIR/foobar-folder\x00" as *const u8 as *const libc::c_char,
            );
        }
        dc_write_file(
            context,
            b"$BLOBDIR/foobar\x00" as *const u8 as *const libc::c_char,
            b"content\x00" as *const u8 as *const libc::c_char as *const libc::c_void,
            7i32 as size_t,
        );
        if 0 != (0
            == dc_file_exist(
                context,
                b"$BLOBDIR/foobar\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                237i32,
                b"dc_file_exist(context, \"$BLOBDIR/foobar\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            != dc_file_exist(
                context,
                b"$BLOBDIR/foobarx\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                238i32,
                b"!dc_file_exist(context, \"$BLOBDIR/foobarx\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(dc_get_filebytes(
            context,
            b"$BLOBDIR/foobar\x00" as *const u8 as *const libc::c_char,
        ) == 7i32 as libc::c_ulonglong) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                239i32,
                b"dc_get_filebytes(context, \"$BLOBDIR/foobar\")==7\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        let abs_path: *mut libc::c_char = dc_mprintf(
            b"%s/%s\x00" as *const u8 as *const libc::c_char,
            context.get_blobdir(),
            b"foobar\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != (0 == dc_is_blobdir_path(context, abs_path)) as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                242i32,
                b"dc_is_blobdir_path(context, abs_path)\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            == dc_is_blobdir_path(
                context,
                b"$BLOBDIR/fofo\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                243i32,
                b"dc_is_blobdir_path(context, \"$BLOBDIR/fofo\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            != dc_is_blobdir_path(
                context,
                b"/BLOBDIR/fofo\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                244i32,
                b"!dc_is_blobdir_path(context, \"/BLOBDIR/fofo\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0 == dc_file_exist(context, abs_path)) as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                245i32,
                b"dc_file_exist(context, abs_path)\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        free(abs_path as *mut libc::c_void);
        if 0 != (0
            == dc_copy_file(
                context,
                b"$BLOBDIR/foobar\x00" as *const u8 as *const libc::c_char,
                b"$BLOBDIR/dada\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                248i32,
                b"dc_copy_file(context, \"$BLOBDIR/foobar\", \"$BLOBDIR/dada\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(dc_get_filebytes(
            context,
            b"$BLOBDIR/dada\x00" as *const u8 as *const libc::c_char,
        ) == 7i32 as libc::c_ulonglong) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                249i32,
                b"dc_get_filebytes(context, \"$BLOBDIR/dada\")==7\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };

        let mut buf: *mut libc::c_void = 0 as *mut libc::c_void;
        let mut buf_bytes: size_t = 0;

        assert_eq!(
            dc_read_file(
                context,
                b"$BLOBDIR/dada\x00" as *const u8 as *const libc::c_char,
                &mut buf,
                &mut buf_bytes,
            ),
            1
        );
        assert_eq!(buf_bytes, 7);
        assert_eq!(
            std::str::from_utf8(std::slice::from_raw_parts(buf as *const u8, buf_bytes)).unwrap(),
            "content"
        );

        free(buf);
        if 0 != (0
            == dc_delete_file(
                context,
                b"$BLOBDIR/foobar\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                258i32,
                b"dc_delete_file(context, \"$BLOBDIR/foobar\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            == dc_delete_file(
                context,
                b"$BLOBDIR/dada\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                259i32,
                b"dc_delete_file(context, \"$BLOBDIR/dada\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            == dc_create_folder(
                context,
                b"$BLOBDIR/foobar-folder\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                261i32,
                b"dc_create_folder(context, \"$BLOBDIR/foobar-folder\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            == dc_file_exist(
                context,
                b"$BLOBDIR/foobar-folder\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                262i32,
                b"dc_file_exist(context, \"$BLOBDIR/foobar-folder\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            == dc_delete_file(
                context,
                b"$BLOBDIR/foobar-folder\x00" as *const u8 as *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                263i32,
                b"dc_delete_file(context, \"$BLOBDIR/foobar-folder\")\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        let fn0: *mut libc::c_char = dc_get_fine_pathNfilename(
            context,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            b"foobar.dadada\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != fn0.is_null() as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                266i32,
                b"fn0\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(strcmp(
            fn0,
            b"$BLOBDIR/foobar.dadada\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                267i32,
                b"strcmp(fn0, \"$BLOBDIR/foobar.dadada\")==0\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        dc_write_file(
            context,
            fn0,
            b"content\x00" as *const u8 as *const libc::c_char as *const libc::c_void,
            7i32 as size_t,
        );
        let fn1: *mut libc::c_char = dc_get_fine_pathNfilename(
            context,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            b"foobar.dadada\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != fn1.is_null() as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                271i32,
                b"fn1\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(strcmp(
            fn1,
            b"$BLOBDIR/foobar-1.dadada\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                272i32,
                b"strcmp(fn1, \"$BLOBDIR/foobar-1.dadada\")==0\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0 == dc_delete_file(context, fn0)) as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                274i32,
                b"dc_delete_file(context, fn0)\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        free(fn0 as *mut libc::c_void);
        free(fn1 as *mut libc::c_void);
    }
    let p1: *mut dc_param_t = dc_param_new();
    dc_param_set_packed(
        p1,
        b"\r\n\r\na=1\nb=2\n\nc = 3 \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(dc_param_get_int(p1, 'a' as i32, 0i32) == 1i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            690i32,
            b"dc_param_get_int(p1, \'a\', 0)==1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_param_get_int(p1, 'b' as i32, 0i32) == 2i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            691i32,
            b"dc_param_get_int(p1, \'b\', 0)==2\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_param_get_int(p1, 'c' as i32, 0i32) == 0i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            692i32,
            b"dc_param_get_int(p1, \'c\', 0)==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_param_exists(p1, 'c' as i32) == 0i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            693i32,
            b"dc_param_exists (p1, \'c\')==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_param_set_int(p1, 'd' as i32, 4i32);
    if 0 != !(dc_param_get_int(p1, 'd' as i32, 0i32) == 4i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            696i32,
            b"dc_param_get_int(p1, \'d\', 0)==4\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_param_empty(p1);
    dc_param_set(
        p1,
        'a' as i32,
        b"foo\x00" as *const u8 as *const libc::c_char,
    );
    dc_param_set_int(p1, 'b' as i32, 2i32);
    dc_param_set(p1, 'c' as i32, 0 as *const libc::c_char);
    dc_param_set_int(p1, 'd' as i32, 4i32);
    if 0 != !(strcmp(
        (*p1).packed,
        b"a=foo\nb=2\nd=4\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            703i32,
            b"strcmp(p1->packed, \"a=foo\\nb=2\\nd=4\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };

    dc_param_set(p1, 'b' as i32, 0 as *const libc::c_char);

    assert_eq!(
        CStr::from_ptr((*p1).packed as *const libc::c_char)
            .to_str()
            .unwrap(),
        "a=foo\nd=4",
    );

    dc_param_set(p1, 'a' as i32, 0 as *const libc::c_char);
    dc_param_set(p1, 'd' as i32, 0 as *const libc::c_char);

    assert_eq!(
        CStr::from_ptr((*p1).packed as *const libc::c_char)
            .to_str()
            .unwrap(),
        "",
    );

    dc_param_unref(p1);

    let mut keys: *mut libc::c_char = dc_get_config(
        context,
        b"sys.config_keys\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(!keys.is_null() && 0 != *keys.offset(0isize) as libc::c_int) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            720i32,
            b"keys && keys[0]\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut sb: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut sb, 200i32);
    dc_strbuilder_catf(
        &mut sb as *mut dc_strbuilder_t,
        b" %s \x00" as *const u8 as *const libc::c_char,
        keys,
    );
    free(keys as *mut libc::c_void);
    keys = sb.buf;
    if 0 != !strstr(
        keys,
        b" probably_never_a_key \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            728i32,
            b"strstr(keys, \" probably_never_a_key \")==NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" addr \x00" as *const u8 as *const libc::c_char).is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            729i32,
            b"strstr(keys, \" addr \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" mail_server \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            730i32,
            b"strstr(keys, \" mail_server \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" mail_user \x00" as *const u8 as *const libc::c_char).is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            731i32,
            b"strstr(keys, \" mail_user \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" mail_pw \x00" as *const u8 as *const libc::c_char).is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            732i32,
            b"strstr(keys, \" mail_pw \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" mail_port \x00" as *const u8 as *const libc::c_char).is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            733i32,
            b"strstr(keys, \" mail_port \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" send_server \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            734i32,
            b"strstr(keys, \" send_server \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" send_user \x00" as *const u8 as *const libc::c_char).is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            735i32,
            b"strstr(keys, \" send_user \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" send_pw \x00" as *const u8 as *const libc::c_char).is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            736i32,
            b"strstr(keys, \" send_pw \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" send_port \x00" as *const u8 as *const libc::c_char).is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            737i32,
            b"strstr(keys, \" send_port \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" server_flags \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            738i32,
            b"strstr(keys, \" server_flags \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" imap_folder \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            739i32,
            b"strstr(keys, \" imap_folder \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" displayname \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            740i32,
            b"strstr(keys, \" displayname \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" selfstatus \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            741i32,
            b"strstr(keys, \" selfstatus \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" selfavatar \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            742i32,
            b"strstr(keys, \" selfavatar \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" e2ee_enabled \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            743i32,
            b"strstr(keys, \" e2ee_enabled \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" mdns_enabled \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            744i32,
            b"strstr(keys, \" mdns_enabled \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" save_mime_headers \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            745i32,
            b"strstr(keys, \" save_mime_headers \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_addr \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            746i32,
            b"strstr(keys, \" configured_addr \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_mail_server \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            747i32,
            b"strstr(keys, \" configured_mail_server \")!=NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_mail_user \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            748i32,
            b"strstr(keys, \" configured_mail_user \")!=NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_mail_pw \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            749i32,
            b"strstr(keys, \" configured_mail_pw \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_mail_port \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            750i32,
            b"strstr(keys, \" configured_mail_port \")!=NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_send_server \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            751i32,
            b"strstr(keys, \" configured_send_server \")!=NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_send_user \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            752i32,
            b"strstr(keys, \" configured_send_user \")!=NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_send_pw \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            753i32,
            b"strstr(keys, \" configured_send_pw \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_send_port \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            754i32,
            b"strstr(keys, \" configured_send_port \")!=NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(
        keys,
        b" configured_server_flags \x00" as *const u8 as *const libc::c_char,
    )
    .is_null() as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            755i32,
            b"strstr(keys, \" configured_server_flags \")!=NULL\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(keys as *mut libc::c_void);

    let mut ok: libc::c_int;
    let mut buf_0: *mut libc::c_char;
    let mut headerline: *const libc::c_char = 0 as *const libc::c_char;
    let mut setupcodebegin: *const libc::c_char = 0 as *const libc::c_char;
    let mut preferencrypt: *const libc::c_char = 0 as *const libc::c_char;
    let mut base64: *const libc::c_char = 0 as *const libc::c_char;
    buf_0 = strdup(
        b"-----BEGIN PGP MESSAGE-----\nNoVal:\n\ndata\n-----END PGP MESSAGE-----\x00" as *const u8
            as *const libc::c_char,
    );
    ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        0 as *mut *const libc::c_char,
        &mut base64,
    );
    if 0 != !(ok == 1i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            821i32,
            b"ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!headerline.is_null()
        && strcmp(
            headerline,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            822i32,
            b"headerline && strcmp(headerline, \"-----BEGIN PGP MESSAGE-----\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };

    assert!(!base64.is_null());
    assert_eq!(
        CStr::from_ptr(base64 as *const libc::c_char)
            .to_str()
            .unwrap(),
        "data",
    );

    free(buf_0 as *mut libc::c_void);

    buf_0 =
        strdup(b"-----BEGIN PGP MESSAGE-----\n\ndat1\n-----END PGP MESSAGE-----\n-----BEGIN PGP MESSAGE-----\n\ndat2\n-----END PGP MESSAGE-----\x00"
                   as *const u8 as *const libc::c_char);
    ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        0 as *mut *const libc::c_char,
        &mut base64,
    );
    if 0 != !(ok == 1i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            828i32,
            b"ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!headerline.is_null()
        && strcmp(
            headerline,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            829i32,
            b"headerline && strcmp(headerline, \"-----BEGIN PGP MESSAGE-----\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };

    assert!(!base64.is_null());
    assert_eq!(
        CStr::from_ptr(base64 as *const libc::c_char)
            .to_str()
            .unwrap(),
        "dat1",
    );

    free(buf_0 as *mut libc::c_void);

    buf_0 = strdup(
        b"foo \n -----BEGIN PGP MESSAGE----- \n base64-123 \n  -----END PGP MESSAGE-----\x00"
            as *const u8 as *const libc::c_char,
    );
    ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        0 as *mut *const libc::c_char,
        &mut base64,
    );
    if 0 != !(ok == 1i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            835i32,
            b"ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!headerline.is_null()
        && strcmp(
            headerline,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            836i32,
            b"headerline && strcmp(headerline, \"-----BEGIN PGP MESSAGE-----\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !setupcodebegin.is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            837i32,
            b"setupcodebegin == NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };

    assert!(!base64.is_null());
    assert_eq!(
        CStr::from_ptr(base64 as *const libc::c_char)
            .to_str()
            .unwrap(),
        "base64-123",
    );

    free(buf_0 as *mut libc::c_void);

    buf_0 = strdup(b"foo-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char);
    ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        0 as *mut *const libc::c_char,
        &mut base64,
    );
    if 0 != !(ok == 0i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            843i32,
            b"ok == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf_0 as *mut libc::c_void);
    buf_0 =
        strdup(b"foo \n -----BEGIN PGP MESSAGE-----\n  Passphrase-BeGIN  :  23 \n  \n base64-567 \r\n abc \n  -----END PGP MESSAGE-----\n\n\n\x00"
                   as *const u8 as *const libc::c_char);
    ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        0 as *mut *const libc::c_char,
        &mut base64,
    );
    if 0 != !(ok == 1i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            848i32,
            b"ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!headerline.is_null()
        && strcmp(
            headerline,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            849i32,
            b"headerline && strcmp(headerline, \"-----BEGIN PGP MESSAGE-----\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!setupcodebegin.is_null()
        && strcmp(
            setupcodebegin,
            b"23\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            850i32,
            b"setupcodebegin && strcmp(setupcodebegin, \"23\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };

    assert!(!base64.is_null());
    assert_eq!(
        CStr::from_ptr(base64 as *const libc::c_char)
            .to_str()
            .unwrap(),
        "base64-567 \n abc",
    );

    free(buf_0 as *mut libc::c_void);

    buf_0 =
        strdup(b"-----BEGIN PGP PRIVATE KEY BLOCK-----\n Autocrypt-Prefer-Encrypt :  mutual \n\nbase64\n-----END PGP PRIVATE KEY BLOCK-----\x00"
                   as *const u8 as *const libc::c_char);
    ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        0 as *mut *const libc::c_char,
        &mut preferencrypt,
        &mut base64,
    );
    if 0 != !(ok == 1i32) as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            856i32,
            b"ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!headerline.is_null()
        && strcmp(
            headerline,
            b"-----BEGIN PGP PRIVATE KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            857i32,
            b"headerline && strcmp(headerline, \"-----BEGIN PGP PRIVATE KEY BLOCK-----\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!preferencrypt.is_null()
        && strcmp(
            preferencrypt,
            b"mutual\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            858i32,
            b"preferencrypt && strcmp(preferencrypt, \"mutual\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };

    assert!(!base64.is_null());
    assert_eq!(
        CStr::from_ptr(base64 as *const libc::c_char)
            .to_str()
            .unwrap(),
        "base64",
    );

    free(buf_0 as *mut libc::c_void);

    let mut norm: *mut libc::c_char = dc_normalize_setup_code(
        context,
        b"123422343234423452346234723482349234\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != norm.is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            868i32,
            b"norm\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        norm,
        b"1234-2234-3234-4234-5234-6234-7234-8234-9234\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            869i32,
            b"strcmp(norm, \"1234-2234-3234-4234-5234-6234-7234-8234-9234\") == 0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(norm as *mut libc::c_void);
    norm = dc_normalize_setup_code(
        context,
        b"\t1 2 3422343234- foo bar-- 423-45 2 34 6234723482349234      \x00" as *const u8
            as *const libc::c_char,
    );
    if 0 != norm.is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            873i32,
            b"norm\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        norm,
        b"1234-2234-3234-4234-5234-6234-7234-8234-9234\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            874i32,
            b"strcmp(norm, \"1234-2234-3234-4234-5234-6234-7234-8234-9234\") == 0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(norm as *mut libc::c_void);
    let mut buf_1: *mut libc::c_char;
    let mut headerline_0: *const libc::c_char = 0 as *const libc::c_char;
    let mut setupcodebegin_0: *const libc::c_char = 0 as *const libc::c_char;
    let mut preferencrypt_0: *const libc::c_char = 0 as *const libc::c_char;
    buf_1 = strdup(S_EM_SETUPFILE);
    if 0 != (0
        == dc_split_armored_data(
            buf_1,
            &mut headerline_0,
            &mut setupcodebegin_0,
            &mut preferencrypt_0,
            0 as *mut *const libc::c_char,
        )) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            883i32,
            b"dc_split_armored_data(buf, &headerline, &setupcodebegin, &preferencrypt, NULL)\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!headerline_0.is_null()
        && strcmp(
            headerline_0,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            884i32,
            b"headerline && strcmp(headerline, \"-----BEGIN PGP MESSAGE-----\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!setupcodebegin_0.is_null()
        && strlen(setupcodebegin_0) < strlen(S_EM_SETUPCODE)
        && strncmp(setupcodebegin_0, S_EM_SETUPCODE, strlen(setupcodebegin_0)) == 0i32)
        as usize
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 885i32,
                     b"setupcodebegin && strlen(setupcodebegin)<strlen(s_em_setupcode) && strncmp(setupcodebegin, s_em_setupcode, strlen(setupcodebegin))==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !preferencrypt_0.is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            886i32,
            b"preferencrypt==NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf_1 as *mut libc::c_void);
    buf_1 = dc_decrypt_setup_file(context, S_EM_SETUPCODE, S_EM_SETUPFILE);
    if 0 != buf_1.is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            889i32,
            b"(buf=dc_decrypt_setup_file(context, s_em_setupcode, s_em_setupfile)) != NULL\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0
        == dc_split_armored_data(
            buf_1,
            &mut headerline_0,
            &mut setupcodebegin_0,
            &mut preferencrypt_0,
            0 as *mut *const libc::c_char,
        )) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            890i32,
            b"dc_split_armored_data(buf, &headerline, &setupcodebegin, &preferencrypt, NULL)\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!headerline_0.is_null()
        && strcmp(
            headerline_0,
            b"-----BEGIN PGP PRIVATE KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            891i32,
            b"headerline && strcmp(headerline, \"-----BEGIN PGP PRIVATE KEY BLOCK-----\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !setupcodebegin_0.is_null() as usize {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            892i32,
            b"setupcodebegin==NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!preferencrypt_0.is_null()
        && strcmp(
            preferencrypt_0,
            b"mutual\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as usize
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            893i32,
            b"preferencrypt && strcmp(preferencrypt, \"mutual\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(buf_1 as *mut libc::c_void);
    if 0 != dc_is_configured(context) {
        let setupcode: *mut libc::c_char;
        let setupfile: *mut libc::c_char;
        setupcode = dc_create_setup_code(context);
        if 0 != setupcode.is_null() as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                901i32,
                b"(setupcode=dc_create_setup_code(context)) != NULL\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(strlen(setupcode) == 44) as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                902i32,
                b"strlen(setupcode) == 44\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(*setupcode.offset(4isize) as libc::c_int == '-' as i32
            && *setupcode.offset(9isize) as libc::c_int == '-' as i32
            && *setupcode.offset(14isize) as libc::c_int == '-' as i32
            && *setupcode.offset(19isize) as libc::c_int == '-' as i32
            && *setupcode.offset(24isize) as libc::c_int == '-' as i32
            && *setupcode.offset(29isize) as libc::c_int == '-' as i32
            && *setupcode.offset(34isize) as libc::c_int == '-' as i32
            && *setupcode.offset(39isize) as libc::c_int == '-' as i32) as usize
        {
            __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                                   &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                         b"../cmdline/stress.c\x00" as *const u8 as
                             *const libc::c_char, 903i32,
                         b"setupcode[4]==\'-\' && setupcode[9]==\'-\' && setupcode[14]==\'-\' && setupcode[19]==\'-\' && setupcode[24]==\'-\' && setupcode[29]==\'-\' && setupcode[34]==\'-\' && setupcode[39]==\'-\'\x00"
                             as *const u8 as *const libc::c_char);
        } else {
        };
        setupfile = dc_render_setup_file(context, setupcode);
        if 0 != setupfile.is_null() as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                905i32,
                b"(setupfile=dc_render_setup_file(context, setupcode)) != NULL\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        let buf_2: *mut libc::c_char = dc_strdup(setupfile);
        let mut headerline_1: *const libc::c_char = 0 as *const libc::c_char;
        let mut setupcodebegin_1: *const libc::c_char = 0 as *const libc::c_char;
        if 0 != (0
            == dc_split_armored_data(
                buf_2,
                &mut headerline_1,
                &mut setupcodebegin_1,
                0 as *mut *const libc::c_char,
                0 as *mut *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                910i32,
                b"dc_split_armored_data(buf, &headerline, &setupcodebegin, NULL, NULL)\x00"
                    as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(!headerline_1.is_null()
            && strcmp(
                headerline_1,
                b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
            ) == 0i32) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                911i32,
                b"headerline && strcmp(headerline, \"-----BEGIN PGP MESSAGE-----\")==0\x00"
                    as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(!setupcodebegin_1.is_null()
            && strlen(setupcodebegin_1) == 2
            && strncmp(setupcodebegin_1, setupcode, 2) == 0i32) as usize
        {
            __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                                   &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                         b"../cmdline/stress.c\x00" as *const u8 as
                             *const libc::c_char, 912i32,
                         b"setupcodebegin && strlen(setupcodebegin)==2 && strncmp(setupcodebegin, setupcode, 2)==0\x00"
                             as *const u8 as *const libc::c_char);
        } else {
        };
        free(buf_2 as *mut libc::c_void);
        let payload: *mut libc::c_char;
        let mut headerline_2: *const libc::c_char = 0 as *const libc::c_char;
        payload = dc_decrypt_setup_file(context, setupcode, setupfile);
        if 0 != payload.is_null() as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                919i32,
                b"(payload=dc_decrypt_setup_file(context, setupcode, setupfile))!=NULL\x00"
                    as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != (0
            == dc_split_armored_data(
                payload,
                &mut headerline_2,
                0 as *mut *const libc::c_char,
                0 as *mut *const libc::c_char,
                0 as *mut *const libc::c_char,
            )) as usize
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                920i32,
                b"dc_split_armored_data(payload, &headerline, NULL, NULL, NULL)\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
        };
        if 0 != !(!headerline_2.is_null()
            && strcmp(
                headerline_2,
                b"-----BEGIN PGP PRIVATE KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
            ) == 0i32) as usize
        {
            __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                                   &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                         b"../cmdline/stress.c\x00" as *const u8 as
                             *const libc::c_char, 921i32,
                         b"headerline && strcmp(headerline, \"-----BEGIN PGP PRIVATE KEY BLOCK-----\")==0\x00"
                             as *const u8 as *const libc::c_char);
        } else {
        };
        free(payload as *mut libc::c_void);
        free(setupfile as *mut libc::c_void);
        free(setupcode as *mut libc::c_void);
    }

    if 0 != dc_is_configured(context) {
        let qr: *mut libc::c_char = dc_get_securejoin_qr(context, 0i32 as uint32_t);
        if 0 != !(strlen(qr) > 55
            && strncmp(
                qr,
                b"OPENPGP4FPR:\x00" as *const u8 as *const libc::c_char,
                12,
            ) == 0i32
            && strncmp(
                &mut *qr.offset(52isize),
                b"#a=\x00" as *const u8 as *const libc::c_char,
                3,
            ) == 0i32) as usize
        {
            __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                                   &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                         b"../cmdline/stress.c\x00" as *const u8 as
                             *const libc::c_char, 1084i32,
                         b"strlen(qr)>55 && strncmp(qr, \"OPENPGP4FPR:\", 12)==0 && strncmp(&qr[52], \"#a=\", 3)==0\x00"
                             as *const u8 as *const libc::c_char);
        } else {
        };
        let mut res: *mut dc_lot_t = dc_check_qr(context, qr);
        if 0 != res.is_null() as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                1087i32,
                b"res\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !((*res).state == 200i32 || (*res).state == 220i32 || (*res).state == 230i32)
            as usize
        {
            __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                                   &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                         b"../cmdline/stress.c\x00" as *const u8 as
                             *const libc::c_char, 1088i32,
                         b"res->state == DC_QR_ASK_VERIFYCONTACT || res->state == DC_QR_FPR_MISMATCH || res->state == DC_QR_FPR_WITHOUT_ADDR\x00"
                             as *const u8 as *const libc::c_char);
        } else {
        };
        dc_lot_unref(res);
        free(qr as *mut libc::c_void);
        res =
            dc_check_qr(context,
                        b"BEGIN:VCARD\nVERSION:3.0\nN:Last;First\nEMAIL;TYPE=INTERNET:stress@test.local\nEND:VCARD\x00"
                            as *const u8 as *const libc::c_char);
        if 0 != res.is_null() as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                1094i32,
                b"res\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !((*res).state == 320i32) as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                1095i32,
                b"res->state == DC_QR_ADDR\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !((*res).id != 0i32 as libc::c_uint) as usize {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                1096i32,
                b"res->id != 0\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        dc_lot_unref(res);
    };
}

#[test]
fn test_encryption_decryption() {
    unsafe {
        let mut bad_data: [libc::c_uchar; 4096] = [0; 4096];
        let mut i_0: libc::c_int = 0i32;
        while i_0 < 4096i32 {
            bad_data[i_0 as usize] = (i_0 & 0xffi32) as libc::c_uchar;
            i_0 += 1
        }
        let mut j: libc::c_int = 0i32;

        while j < 4096 / 40 {
            let bad_key = Key::from_binary(
                &mut *bad_data.as_mut_ptr().offset(j as isize) as *const u8,
                4096 / 2 + j,
                if 0 != j & 1 {
                    KeyType::Public
                } else {
                    KeyType::Private
                },
            );

            assert!(bad_key.is_none());
            j += 1
        }

        let (public_key, private_key) =
            dc_pgp_create_keypair(b"foo@bar.de\x00" as *const u8 as *const libc::c_char).unwrap();

        private_key.split_key().unwrap();

        let (public_key2, private_key2) =
            dc_pgp_create_keypair(b"two@zwo.de\x00" as *const u8 as *const libc::c_char).unwrap();

        assert_ne!(public_key, public_key2);

        let original_text: *const libc::c_char =
            b"This is a test\x00" as *const u8 as *const libc::c_char;
        let mut keyring = Keyring::default();
        keyring.add_owned(public_key.clone());
        keyring.add_ref(&public_key2);

        let ctext = dc_pgp_pk_encrypt(
            original_text as *const libc::c_void,
            strlen(original_text),
            &keyring,
            Some(&private_key),
        )
        .unwrap();

        assert!(!ctext.is_empty());
        assert!(ctext.starts_with("-----BEGIN PGP MESSAGE-----"));

        let ctext_signed_bytes = ctext.len();
        let ctext_signed = CString::new(ctext).unwrap();

        let ctext = dc_pgp_pk_encrypt(
            original_text as *const libc::c_void,
            strlen(original_text),
            &keyring,
            None,
        )
        .unwrap();
        assert!(!ctext.is_empty());
        assert!(ctext.starts_with("-----BEGIN PGP MESSAGE-----"));

        let ctext_unsigned_bytes = ctext.len();
        let ctext_unsigned = CString::new(ctext).unwrap();

        let mut keyring = Keyring::default();
        keyring.add_owned(private_key);

        let mut public_keyring = Keyring::default();
        public_keyring.add_ref(&public_key);

        let mut public_keyring2 = Keyring::default();
        public_keyring2.add_owned(public_key2.clone());

        let mut valid_signatures: HashSet<String> = Default::default();

        let plain = dc_pgp_pk_decrypt(
            ctext_signed.as_ptr() as *const _,
            ctext_signed_bytes,
            &keyring,
            &public_keyring,
            Some(&mut valid_signatures),
        )
        .unwrap();

        assert_eq!(
            std::str::from_utf8(&plain).unwrap(),
            CStr::from_ptr(original_text).to_str().unwrap()
        );
        assert_eq!(valid_signatures.len(), 1);

        valid_signatures.clear();

        let empty_keyring = Keyring::default();
        let plain = dc_pgp_pk_decrypt(
            ctext_signed.as_ptr() as *const _,
            ctext_signed_bytes,
            &keyring,
            &empty_keyring,
            Some(&mut valid_signatures),
        )
        .unwrap();
        assert_eq!(
            std::str::from_utf8(&plain).unwrap(),
            CStr::from_ptr(original_text).to_str().unwrap()
        );
        assert_eq!(valid_signatures.len(), 0);

        valid_signatures.clear();

        let plain = dc_pgp_pk_decrypt(
            ctext_signed.as_ptr() as *const _,
            ctext_signed_bytes,
            &keyring,
            &public_keyring2,
            Some(&mut valid_signatures),
        )
        .unwrap();
        assert_eq!(
            std::str::from_utf8(&plain).unwrap(),
            CStr::from_ptr(original_text).to_str().unwrap()
        );
        assert_eq!(valid_signatures.len(), 0);

        valid_signatures.clear();

        public_keyring2.add_ref(&public_key);

        let plain = dc_pgp_pk_decrypt(
            ctext_signed.as_ptr() as *const _,
            ctext_signed_bytes,
            &keyring,
            &public_keyring2,
            Some(&mut valid_signatures),
        )
        .unwrap();
        assert_eq!(
            std::str::from_utf8(&plain).unwrap(),
            CStr::from_ptr(original_text).to_str().unwrap()
        );
        assert_eq!(valid_signatures.len(), 1);

        valid_signatures.clear();

        let plain = dc_pgp_pk_decrypt(
            ctext_unsigned.as_ptr() as *const _,
            ctext_unsigned_bytes,
            &keyring,
            &public_keyring,
            Some(&mut valid_signatures),
        )
        .unwrap();
        assert_eq!(
            std::str::from_utf8(&plain).unwrap(),
            CStr::from_ptr(original_text).to_str().unwrap()
        );

        valid_signatures.clear();

        let mut keyring = Keyring::default();
        keyring.add_ref(&private_key2);
        let mut public_keyring = Keyring::default();
        public_keyring.add_ref(&public_key);

        let plain = dc_pgp_pk_decrypt(
            ctext_signed.as_ptr() as *const _,
            ctext_signed_bytes,
            &keyring,
            &public_keyring,
            None,
        )
        .unwrap();
        assert_eq!(
            std::str::from_utf8(&plain).unwrap(),
            CStr::from_ptr(original_text).to_str().unwrap()
        );
    }
}

unsafe extern "C" fn cb(
    _context: &Context,
    _event: Event,
    _data1: uintptr_t,
    _data2: uintptr_t,
) -> uintptr_t {
    0
}

#[allow(dead_code)]
struct TestContext {
    ctx: Context,
    dir: TempDir,
}

unsafe fn create_test_context() -> TestContext {
    let mut ctx = dc_context_new(cb, std::ptr::null_mut(), std::ptr::null_mut());
    let dir = tempdir().unwrap();
    let dbfile = CString::new(dir.path().join("db.sqlite").to_str().unwrap()).unwrap();
    assert_eq!(
        dc_open(&mut ctx, dbfile.as_ptr(), std::ptr::null()),
        1,
        "Failed to open {}",
        CStr::from_ptr(dbfile.as_ptr() as *const libc::c_char)
            .to_str()
            .unwrap()
    );

    TestContext { ctx: ctx, dir: dir }
}

#[test]
fn test_dc_kml_parse() {
    unsafe {
        let context = create_test_context();

        let xml: *const libc::c_char =
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document addr=\"user@example.org\">\n<Placemark><Timestamp><when>2019-03-06T21:09:57Z</when></Timestamp><Point><coordinates accuracy=\"32.000000\">9.423110,53.790302</coordinates></Point></Placemark>\n<PlaceMARK>\n<Timestamp><WHEN > \n\t2018-12-13T22:11:12Z\t</wHeN></Timestamp><Point><coordinates aCCuracy=\"2.500000\"> 19.423110 \t , \n 63.790302\n </coordinates></Point></Placemark>\n</Document>\n</kml>\x00"
            as *const u8 as *const libc::c_char;

        let kml: *mut dc_kml_t = dc_kml_parse(&context.ctx, xml, strlen(xml));

        assert!(!(*kml).addr.is_null());
        assert_eq!(
            CStr::from_ptr((*kml).addr as *const libc::c_char)
                .to_str()
                .unwrap(),
            "user@example.org",
        );

        assert_eq!(dc_array_get_cnt((*kml).locations), 2);

        assert!(dc_array_get_latitude((*kml).locations, 0) > 53.6f64);
        assert!(dc_array_get_latitude((*kml).locations, 0) < 53.8f64);
        assert!(dc_array_get_longitude((*kml).locations, 0) > 9.3f64);
        assert!(dc_array_get_longitude((*kml).locations, 0) < 9.5f64);
        assert!(dc_array_get_accuracy((*kml).locations, 0) > 31.9f64);
        assert!(dc_array_get_accuracy((*kml).locations, 0) < 32.1f64);
        assert_eq!(dc_array_get_timestamp((*kml).locations, 0), 1551906597);

        assert!(dc_array_get_latitude((*kml).locations, 1) > 63.6f64);
        assert!(dc_array_get_latitude((*kml).locations, 1) < 63.8f64);
        assert!(dc_array_get_longitude((*kml).locations, 1) > 19.3f64);
        assert!(dc_array_get_longitude((*kml).locations, 1) < 19.5f64);
        assert!(dc_array_get_accuracy((*kml).locations, 1) > 2.4f64);
        assert!(dc_array_get_accuracy((*kml).locations, 1) < 2.6f64);

        assert_eq!(dc_array_get_timestamp((*kml).locations, 1), 1544739072);

        dc_kml_unref(kml);
    }
}

#[test]
fn test_dc_mimeparser_with_context() {
    unsafe {
        let context = create_test_context();

        let mut mimeparser = dc_mimeparser_new(&context.ctx);
        let raw: *const libc::c_char =
        b"Content-Type: multipart/mixed; boundary=\"==break==\";\nSubject: outer-subject\nX-Special-A: special-a\nFoo: Bar\nChat-Version: 0.0\n\n--==break==\nContent-Type: text/plain; protected-headers=\"v1\";\nSubject: inner-subject\nX-Special-B: special-b\nFoo: Xy\nChat-Version: 1.0\n\ntest1\n\n--==break==--\n\n\x00"
            as *const u8 as *const libc::c_char;

        dc_mimeparser_parse(&mut mimeparser, raw, strlen(raw));
        assert_eq!(
            CStr::from_ptr(mimeparser.subject as *const libc::c_char)
                .to_str()
                .unwrap(),
            "inner-subject",
        );

        let mut of: *mut mailimf_optional_field = dc_mimeparser_lookup_optional_field(
            &mimeparser,
            b"X-Special-A\x00" as *const u8 as *const libc::c_char,
        );
        assert_eq!(
            CStr::from_ptr((*of).fld_value as *const libc::c_char)
                .to_str()
                .unwrap(),
            "special-a",
        );

        of = dc_mimeparser_lookup_optional_field(
            &mimeparser,
            b"Foo\x00" as *const u8 as *const libc::c_char,
        );
        assert_eq!(
            CStr::from_ptr((*of).fld_value as *const libc::c_char)
                .to_str()
                .unwrap(),
            "Bar",
        );

        of = dc_mimeparser_lookup_optional_field(
            &mimeparser,
            b"Chat-Version\x00" as *const u8 as *const libc::c_char,
        );
        assert_eq!(
            CStr::from_ptr((*of).fld_value as *const libc::c_char)
                .to_str()
                .unwrap(),
            "1.0",
        );
        assert_eq!(carray_count(mimeparser.parts), 1);

        dc_mimeparser_unref(&mut mimeparser);
    }
}

#[test]
fn test_dc_get_oauth2_url() {
    let ctx = unsafe { create_test_context() };
    let addr = "dignifiedquire@gmail.com";
    let redirect_uri = "chat.delta:/com.b44t.messenger";
    let res = dc_get_oauth2_url(&ctx.ctx, addr, redirect_uri);

    assert_eq!(res, Some("https://accounts.google.com/o/oauth2/auth?client_id=959970109878-4mvtgf6feshskf7695nfln6002mom908.apps.googleusercontent.com&redirect_uri=chat.delta:/com.b44t.messenger&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F%20email&access_type=offline".into()));
}

#[test]
fn test_dc_get_oauth2_addr() {
    let ctx = unsafe { create_test_context() };
    let addr = "dignifiedquire@gmail.com";
    let code = "fail";
    let res = dc_get_oauth2_addr(&ctx.ctx, addr, code);
    // this should fail as it is an invalid password
    assert_eq!(res, None);
}

#[test]
fn test_dc_get_oauth2_token() {
    let ctx = unsafe { create_test_context() };
    let addr = "dignifiedquire@gmail.com";
    let code = "fail";
    let res = dc_get_oauth2_access_token(&ctx.ctx, addr, code, 0);
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
fn test_arr_to_string() {
    let arr2: [uint32_t; 4] = [
        0i32 as uint32_t,
        12i32 as uint32_t,
        133i32 as uint32_t,
        1999999i32 as uint32_t,
    ];

    let str_0 = unsafe { dc_arr_to_string(arr2.as_ptr(), 4i32) };
    assert_eq!(to_string(str_0), "0,12,133,1999999");
    unsafe { free(str_0 as *mut _) };
}
