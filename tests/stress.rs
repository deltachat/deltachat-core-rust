//! Stress some functions for testing; if used as a lib, this file is obsolete.

use std::ffi::{CStr, CString};
use tempfile::tempdir;

use deltachat::constants::*;
use deltachat::dc_aheader::*;
use deltachat::dc_array::*;
use deltachat::dc_configure::*;
use deltachat::dc_contact::*;
use deltachat::dc_context::*;
use deltachat::dc_hash::*;
use deltachat::dc_imex::*;
use deltachat::dc_key::*;
use deltachat::dc_keyring::*;
use deltachat::dc_location::*;
use deltachat::dc_lot::*;
use deltachat::dc_mimeparser::*;
use deltachat::dc_msg::*;
use deltachat::dc_param::*;
use deltachat::dc_pgp::*;
use deltachat::dc_qr::*;
use deltachat::dc_saxparser::*;
use deltachat::dc_securejoin::*;
use deltachat::dc_simplify::*;
use deltachat::dc_strbuilder::*;
use deltachat::dc_strencode::*;
use deltachat::dc_tools::*;
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
    b"-----BEGIN PGP MESSAGE-----\nPassphrase-Format: numeric9x4\nPassphrase-Begin: 17\n\nwy4ECQMI0jNRBQfVKHVg1+a2Yihd6JAjR9H0kk3oDVeX7nc4Oi+IjEtonUJt\nPQpO0tPWASWYuYvjZSuTz9r1yZYV+y4mu9bu9NEQoRlWg2wnbjoUoKk4emFF\nFweUj84iI6VWTCSRyMu5d5JS1RfOdX4CG/muLAegyIHezqYOEC0Z3b9Ci9rd\nDiSgqqN+/LDkUR/vr7L2CSLN5suBP9Hsz75AtaV8DJ2DYDywYX89yH1CfL1O\nWohyrJPdmGJZfdvQX0LI9mzN7MH0W6vUJeCaUpujc+UkLiOM6TDB74rmYF+V\nZ7K9BXbaN4V6dyxVZfgpXUoZlaNpvqPJXuLHJ68umkuIgIyQvzmMj3mFgZ8s\nakCt6Cf3o5O9n2PJvX89vuNnDGJrO5booEqGaBJfwUk0Rwb0gWsm5U0gceUz\ndce8KZK15CzX+bNv5OC+8jjjBw7mBHVt+2q8LI+G9fEy9NIREkp5/v2ZRN0G\nR6lpZwW+8TkMvJnriQeABqDpxsJVT6ENYAhkPG3AZCr/whGBU3EbDzPexXkz\nqt8Pdu5DrazLSFtjpjkekrjCh43vHjGl8IOiWxKQx0VfBkHJ7O9CsHmb0r1o\nF++fMh0bH1/aewmlg5wd0ixwZoP1o79he8Q4kfATZAjvB1xSLyMma+jxW5uu\nU3wYUOsUmYmzo46/QzizFCUpaTJ4ZQZY1/4sflidsl/XgZ0fD1NCrdkWBNA1\n0tQF949pEAeA4hSfHfQDNKAY8A7fk8lZblqWPkyu/0x8eV537QOhs89ZvhSB\nV87KEAwxWt60+Eolf8PvvkvB/AKlfWq4MYShgyldwwCfkED3rv2mvTsdqfvW\nWvqZNo4eRkJrnv9Be3LaXoFyY6a3z+ObBIkKI+u5azGJYge97O4E2DrUEKdQ\ncScq5upzXity0E+Yhm964jzBzxnA52S4RoXzkjTxH+AHjQ5+MHQxmRfMd2ly\n7skM106weVOR0JgOdkvfiOFDTHZLIVCzVyYVlOUJYYwPhmM1426zbegHNkaM\nM2WgvjMp5G+X9qfDWKecntQJTziyDFZKfd1UrUCPHrvl1Ac9cuqgcCXLtdUS\njI+e1Y9fXvgyvHiMX0ztSz1yfvnRt34508G9j68fEQFQR/VIepULB5/SqKbq\np2flgJL48kY32hEw2GRPri64Tv3vMPIWa//zvQDhQPmcd3S4TqnTIIKUoTAO\nNUo6GS9UAX12fdSFPZINcAkNIaB69+iwGyuJE4FLHKVkqNnNmDwF3fl0Oczo\nhbboWzA3GlpR2Ri6kfe0SocfGR0CHT5ZmqI6es8hWx+RN8hpXcsRxGS0BMi2\nmcJ7fPY+bKastnEeatP+b0XN/eaJAPZPZSF8PuPeQ0Uc735fylPrrgtWK9Gp\nWq0DPaWV/+O94OB/JvWT5wq7d/EEVbTck5FPl4gdv3HHpaaQ6/8G89wVMEXA\nGUxB8WuvNeHAtQ7qXF7TkaZvUpF0rb1aV88uABOOPpsfAyWJo/PExCZacg8R\nGOQYI6inV5HcGUw06yDSqArHZmONveqjbDBApenearcskv6Uz7q+Bp60GGSA\nlvU3C3RyP/OUc1azOp72MIe0+JvP8S5DN9/Ltc/5ZyZHOjLoG+npIXnThYwV\n0kkrlsi/7loCzvhcWOac1vrSaGVCfifkYf+LUFQFrFVbxKLOQ6vTsYZWM0yM\nQsMMywW5A6CdROT5UB0UKRh/S1cwCwrN5UFTRt2UpDF3wSBAcChsHyy90RAL\nXd4+ZIyf29GIFuwwQyzGBWnXQ2ytU4kg/D5XSqJbJJTya386UuyQpnFjI19R\nuuD0mvEfFvojCKDJDWguUNtWsHSg01NXDSrY26BhlOkMpUrzPfX5r0FQpgDS\nzOdY9SIG+y9MKG+4nwmYnFM6V5NxVL+6XZ7BQTvlLIcIIu+BujVNWteDnWNZ\nT1UukCGmFd8sNZpCc3wu4o/gLDQxih/545tWMf0dmeUfYhKcjSX9uucMRZHT\n1N0FINw04fDdp2LccL+WCGatFGnkZVPw3asid4d1od9RG9DbNRBJEp/QeNhc\n/peJCPLGYlA1NjTEq+MVB+DHdGNOuy//be3KhedBr6x4VVaDzL6jyHu/a7PR\nBWRVtI1CIVDxyrEXucHdGQoEm7p+0G2zouOe/oxbPFoEYrjaI+0e/FN3u/Y3\naG0dlYWbxeHMqTh2F3lB/CFALReeGqqN6PwRyePWKaVctZYb6ydf9JVl6q1/\naV9C5rf9eFGqqA+OIx/+XuAG1w0rwlznvtajHzCoUeA4QfbmuOV/t5drWN2N\nPCk2mJlcSmd7lx53rnOIgme1hggchjezc4TisL4PvSLxjJ7DxzktD2jv2I/Q\nOlSxTUaXnGfIVedsI0WjFomz5w9tZjC0B5O5TpSRRz6gfpe/OC3kV7qs1YCS\nlJTTxj1mTs6wqt0WjKkN/Ke0Cm5r7NQ79szDNlcC0AViEOQb3U1R88nNdiVx\nymKT5Dl+yM6acv53lNX6O5BH+mpP2/pCpi3x+kYFyr4cUsNgVVGlhmkPWctZ\ntrHvO7wcLrAsrLNqRxt1G3DLjQt9VY+w5qOPJv6s9qd5JBL/qtH5zqIXiXlM\nIWI9LLwHFFXqjk/f6G4LyOeHB9AqccGQ4IztgzTKmYEmFWVIpTO4UN6+E7yQ\ngtcYSIUEJo824ht5rL+ODqmCSAWsWIomEoTPvgn9QqO0YRwAEMpsFtE17klS\nqjbYyV7Y5A0jpCvqbnGmZPqCgzjjN/p5VKSNjSdM0vdwBRgpXlyooXg/EGoJ\nZTZH8nLSuYMMu7AK8c7DKJ1AocTNYHRe9xFV8RzEiIm3zaezxa0r+Fo3nuTX\nUR9DOH0EHaDLrFQcfS5y1iRxY9CHg0N2ECaUzr/H7jck9mLZ7v9xisj3QDuv\ni0xQbC4BTxMEBGTK8fOcjHHOABOyhqotOreERqwOV2c1OOGUQE8QK18zJCUd\nBTmQZ709ttASD7VWK4TraOGczZXkZsKdZko5T6+6EkFy9H+gwENLUG9zk0x9\n2G5zicDr6PDoAGDuoB3B3VA8ertXTX7zEz30N6m+tcAtPWka0owokLy3f0o7\nZdytBPkly8foTMWKF2vsJ8K4Xdn/57jJ2qFku32xmtiPIoa6s8wINO06AVB0\n0/AuttvxcPr+ycE+9wRZHx6JBujAqOZztU3zu8WZMaqVKb7gnmkWPiL+1XFp\n2+mr0AghScIvjzTDEjigDtLydURJrW01wXjaR0ByBT4z8ZjaNmQAxIPOIRFC\nbD0mviaoX61qgQLmSc6mzVlzzNZRCKtSvvGEK5NJ6CB6g2EeFau8+w0Zd+vv\n/iv6Img3pUBgvpMaIsxRXvGZwmo2R0tztJt+CqHRvyTWjQL+CjIAWyoHEdVH\nk7ne/q9zo3iIMsQUO7tVYtgURpRYc2OM1IVQtrgbmbYGEdOrhMjaWULg9C7o\n6oDM0EFlCAId3P8ykXQNMluFKlf9il5nr19B/qf/wh6C7DFLOmnjTWDXrEiP\n6wFEWTeUWLchGlbpiJFEu05MWPIRoRd3BHQvVpzLLgeBdxMVW7D6WCK+KJxI\nW1rOKhhLVvKU3BrFgr12A4uQm+6w1j33Feh68Y0JB7GLDBBGe11QtLCD6kz5\nRzFl+GbgiwpHi3nlCc5yiNwyPq/JRxU3GRb62YJcsSQBg+CD3Mk5FGiDcuvp\nkZXOcTE2FAnUDigjEs+oH2qkhD4/5CiHkrfFJTzv+wqw+jwxPor2jkZH2akN\n6PssXQYupXJE3NmcyaYT+b5E6qbkIyQj7CknkiqmrqrmxkOQxA+Ab2Vy9zrW\nu0+Wvf+C+SebWTo3qfJZQ3KcASZHa5AGoSHetWzH2fNLIHfULXac/T++1DWE\nnbeNvhXiFmAJ+BRsZj9p6RcnSamk4bjAbX1lg2G3Sq6MiA1fIRSMlSjuDLrQ\n8xfVFrg7gfBIIQPErJWv2GdAsz76sLxuSXQLKYpFnozvMT7xRs84+iRNWWh9\nSNibbEjlh0DcJlKw49Eis/bN22sDQWy4awHuRvvQetk/QCgp54epuqWnbxoE\nXZDgGBBkMc3or+6Cxr3q9x7J/oHLvPb+Q5yVP9fyz6ZiSVWluMefA9smjJ/A\nKMD84s7uO/8/4yug+swXGrcBjHSddTcy05vm+7X6o9IEZKZb5tz7VqAfEcuk\nQNPUWCMudhzxSNr4+yVXRVpcjsjKtplJcXC5aIuJwq3C5OdysCGqXWjLuUu1\nOFSoPvTsYC2VxYdFUcczeHEFTxXoXz3I0TyLPyxUNsJiKpUGt/SXmV/IyAx+\nh6pZ2OUXspC9d78DdiHZtItPjEGiIb678ZyMxWPE59XQd/ad92mlPHU8InXD\nyTq6otZ7LwAOLGbDR9bqN7oX8PCHRwuu30hk2b4+WkZn/WLd2KCPddQswZJg\nQgi5ajUaFhZvxF5YNTqIzzYVh7Y8fFMfzH9AO+SJqy+0ECX0GwtHHeVsXYNb\nP/NO/ma4MI8301JyipPmdtzvvt9NOD/PJcnZH2KmDquARXMO/vKbn3rNUXog\npTFqqyNTr4L5FK86QPEoE4hDy9ItHGlEuiNVD+5suGVGUgYfV7AvZU46EeqO\nrfFj8wNSX1aK/pIwWmh1EkygPSxomWRUANLX1jO6zX9wk2X80Xn9q/8jot1k\nVl54OOd7cvGls2wKkEZi5h3p6KKZHJ+WIDBQupeJbuma1GK8wAiwjDH59Y0X\nwXHAk7XA+t4u0dgRpZbUUMqQmvEvfJaCr4qMlpuGdEYbbpIMUB1qCfYU9taL\nzbepMIT+XYD5mTyytZhR+zrsfpt1EzbrhuabqPioySoIS/1+bWfxvndq16r0\nAdNxR5LiVSVh8QJr3B/HJhVghgSVrrynniG3E94abNWL/GNxPS/dTHSf8ass\nvbv7+uznADzHsMiG/ZlLAEkQJ9j0ENJvHmnayeVFIXDV6jPCcQJ+rURDgl7z\n/qTLfe3o3zBMG78LcB+xDNXTQrK5Z0LX7h17hLSElpiUghFa9nviCsT0nkcr\nnz302P4IOFwJuYMMCEfW+ywTn+CHpKjLHWkZSZ4q6LzNTbbgXZn/vh7njNf0\nQHaHmaMNxnDhUw/Bl13uM52qtsfEYK07SEhLFlJbAk0G7q+OabK8dJxCRwS3\nX9k4juzLUYhX8XBovg9G3YEVckb6iM8/LF/yvNXbUsPrdhYU9lPA63xD0Pgb\nzthZCLIlnF+lS6e41WJv3n1dc4dFWD7F5tmt/7uwLC6oUGYsccSzY+bUkYhL\ndp7tlQRd5AG/Xz8XilORk8cUjvi6uZss5LyQpKvGSU+77C8ZV/oS62BdS5TE\nosBTrO2/9FGzQtHT+8DJSTPPgR6rcQUWLPemiG09ACKfRQ/g3b9Qj0upOcKL\n6dti0lq7Aorc39vV18DPMFBOwzchUEBlBFyuSa4AoD30tsoilAC3qbzBwu3z\nQLjmst76HEcWDkxgDAhlBz6/XgiVZsCivn7ygigmc2+hNEzIdDsKKfM9bkoe\n3uJzmmsv8Bh5ZEtfGoGNmu/zA7tgvTOCBeotYeHr2O6pLmYb3hK+E/qCBl14\n8pK4qYrjAlF+ZMq9BzXcaz5mRfKVfAQtghHOaNqopBczSE1bjFF6HaNhIaGa\nN8YdabNQG7mLI/fgBxJfkPl6HdIhEpctp4RURbSFhW+wn0o85VyHM6a+6Vgj\nNrYmhxPZ6N1KN0Qy76aNiw7nAToRRcOv87uZnkDIeVH8mP/0hldyiy/Y97cG\nQgOeQHOG27QW57nHhqLRqvf0zzQZekuXWFbqajpaabEcdGXyiUpJ8/ZopBPM\nAJwfkyA2LkV946IA4JV6sPnu9pYzpXQ4vdQKJ6DoDUyRTQmgmfSFGtfHAozY\nV9k0iQeetSkYYtOagTrg3t92v7M00o/NJW/rKX4jj2djD8wtBovOcv4kxg4Z\no58Iv94ROim48XfyesvSYKN1xqqbXH4sfE6b4b9pLUxQVOmWANLK9MK8D+Ci\nIvrGbz5U5bZP6vlNbe9bYzjvWTPjaMrjXknRTBcikavqOfDTSIVFtT4qvhvK\n42PpOrm0qdiLwExGKQ9FfEfYZRgEcYRGg7rH3oNz6ZNOEXppF3tCl9yVOlFb\nygdIeT3Z3HeOQbAsi8jK7o16DSXL7ZOpFq9Bv9yzusrF7Eht/fSEpAVUO3D1\nIuqjZcsQRhMtIvnF0oFujFtooJx9x3dj/RarvEGX/NzwATZkgJ+yWs2etruA\nEzMQqED4j7Lb790zEWnt+nuHdCdlPnNy8RG5u5X62p3h5KqUbg9HfmIuuESi\nhwr6dKsVQGc5XUB5KTt0dtjWlK5iaetDsZFuF5+aE0Xa6PmiQ2e7ZPFyxXmO\nT/PSHzobx0qClKCu+tSWA1HDSL08IeoGZEyyhoaxyn5D9r1Mqg101v/iu59r\nlRRs+plAhbuq5aQA3WKtF1N6Zb5+AVRpNUyrxyHoH36ddR4/n7lnIld3STGD\nRqZLrOuKHS3dCNW2Pt15lU+loYsWFZwC6T/tAbvwhax+XaBMiKQSDFmG9sBw\nTiM1JWXhq2IsjXBvCl6k2AKWLQOvc/Hin+oYs4d7M9mi0vdoEOAMadU/+Pqn\nuZzP941mOUV5UeTCCbjpyfI7qtIi3TH1cQmC2kG2HrvQYuM6Momp//JusH1+\n9eHgFo25HbitcKJ1sAqxsnYIW5/jIVyIJC7tatxmNfFQQ/LUb2cT+Jowwsf4\nbbPinA9S6aQFy9k3vk07V2ouYl+cpMMXmNAUrboFRLxw7QDapWYMKdmnbU5O\nHZuDz3iyrm0lMPsRtt/f5WUhZYY4vXT5/dj+8P6Pr5fdc4S84i5qEzf7bX/I\nSc6fpISdYBscfHdv6uXsEVtVPKEuQVYwhyc4kkwVKjZBaqsgjAA7VEhQXzO3\nrC7di4UhabWQCQTG1GYZyrj4bm6dg/32uVxMoLS5kuSpi3nMz5JmQahLqRxh\nargg13K2/MJ7w2AI23gCvO5bEmD1ZXIi1aGYdZfu7+KqrTumYxj0KgIesgU0\n6ekmPh4Zu5lIyKopa89nfQVj3uKbwr9LLHegfzeMhvI5WQWghKcNcXEvJwSA\nvEik5aXm2qSKXT+ijXBy5MuNeICoGaQ5WA0OJ30Oh5dN0XpLtFUWHZKThJvR\nmngm1QCMMw2v/j8=\n=9sJE\n-----END PGP MESSAGE-----\n\x00"
        as *const u8 as *const libc::c_char;

unsafe extern "C" fn stress_functions(context: &dc_context_t) {
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

    let mut simplify: *mut dc_simplify_t = dc_simplify_new();
    let mut html: *const libc::c_char =
        b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
    let mut plain: *mut libc::c_char =
        dc_simplify_simplify(simplify, html, strlen(html) as libc::c_int, 1i32, 0i32);

    assert_eq!(
        CStr::from_ptr(plain as *const libc::c_char)
            .to_str()
            .unwrap(),
        "line1\nline2",
    );
    free(plain as *mut libc::c_void);

    html = b"<a href=url>text</a\x00" as *const u8 as *const libc::c_char;
    plain = dc_simplify_simplify(simplify, html, strlen(html) as libc::c_int, 1i32, 0i32);
    if 0 != !(strcmp(
        plain,
        b"[text](url)\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            179i32,
            b"strcmp(plain, \"[text](url)\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(plain as *mut libc::c_void);
    html =
        b"<!DOCTYPE name [<!DOCTYPE ...>]><!-- comment -->text <b><?php echo ... ?>bold</b><![CDATA[<>]]>\x00"
            as *const u8 as *const libc::c_char;
    plain = dc_simplify_simplify(simplify, html, strlen(html) as libc::c_int, 1i32, 0i32);
    if 0 != !(strcmp(
        plain,
        b"text *bold*<>\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            184i32,
            b"strcmp(plain, \"text *bold*<>\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(plain as *mut libc::c_void);
    html =
        b"&lt;&gt;&quot;&apos;&amp; &auml;&Auml;&ouml;&Ouml;&uuml;&Uuml;&szlig; foo&AElig;&ccedil;&Ccedil; &diams;&noent;&lrm;&rlm;&zwnj;&zwj;\x00"
            as *const u8 as *const libc::c_char;
    plain = dc_simplify_simplify(simplify, html, strlen(html) as libc::c_int, 1i32, 0i32);
    if 0 !=
           !(strcmp(plain,
                    b"<>\"\'& \xc3\xa4\xc3\x84\xc3\xb6\xc3\x96\xc3\xbc\xc3\x9c\xc3\x9f foo\xc3\x86\xc3\xa7\xc3\x87 \xe2\x99\xa6&noent;\x00"
                        as *const u8 as *const libc::c_char) == 0i32) as
               libc::c_int as libc::c_long {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 189i32,
                     b"strcmp(plain, \"<>\\\"\'& \xc3\xa4\xc3\x84\xc3\xb6\xc3\x96\xc3\xbc\xc3\x9c\xc3\x9f foo\xc3\x86\xc3\xa7\xc3\x87 \xe2\x99\xa6&noent;\")==0\x00"
                         as *const u8 as *const libc::c_char);
    } else { };
    free(plain as *mut libc::c_void);
    dc_simplify_unref(simplify);
    let mut xml: *const libc::c_char =
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document addr=\"user@example.org\">\n<Placemark><Timestamp><when>2019-03-06T21:09:57Z</when></Timestamp><Point><coordinates accuracy=\"32.000000\">9.423110,53.790302</coordinates></Point></Placemark>\n<PlaceMARK>\n<Timestamp><WHEN > \n\t2018-12-13T22:11:12Z\t</wHeN></Timestamp><Point><coordinates aCCuracy=\"2.500000\"> 19.423110 \t , \n 63.790302\n </coordinates></Point></Placemark>\n</Document>\n</kml>\x00"
            as *const u8 as *const libc::c_char;
    let mut kml: *mut dc_kml_t = dc_kml_parse(context, xml, strlen(xml));
    if 0 != !(!(*kml).addr.is_null()
        && strcmp(
            (*kml).addr,
            b"user@example.org\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            205i32,
            b"kml->addr && strcmp(kml->addr, \"user@example.org\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_array_get_cnt((*kml).locations) == 2) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            206i32,
            b"dc_array_get_cnt(kml->locations)==2\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut lat: libc::c_double = dc_array_get_latitude((*kml).locations, 0i32 as size_t);
    if 0 != !(lat > 53.6f64 && lat < 53.8f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            208i32,
            b"lat>53.6 && lat<53.8\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut lng: libc::c_double = dc_array_get_longitude((*kml).locations, 0i32 as size_t);
    if 0 != !(lng > 9.3f64 && lng < 9.5f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            209i32,
            b"lng> 9.3 && lng< 9.5\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut acc: libc::c_double = dc_array_get_accuracy((*kml).locations, 0i32 as size_t);
    if 0 != !(acc > 31.9f64 && acc < 32.1f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            210i32,
            b"acc>31.9 && acc<32.1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_array_get_timestamp((*kml).locations, 0i32 as size_t)
        == 1551906597i32 as libc::c_long) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            211i32,
            b"dc_array_get_timestamp(kml->locations, 0)==1551906597\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    lat = dc_array_get_latitude((*kml).locations, 1i32 as size_t);
    if 0 != !(lat > 63.6f64 && lat < 63.8f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            213i32,
            b"lat>63.6 && lat<63.8\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    lng = dc_array_get_longitude((*kml).locations, 1i32 as size_t);
    if 0 != !(lng > 19.3f64 && lng < 19.5f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            214i32,
            b"lng>19.3 && lng<19.5\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    acc = dc_array_get_accuracy((*kml).locations, 1i32 as size_t);
    if 0 != !(acc > 2.4f64 && acc < 2.6f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            215i32,
            b"acc> 2.4 && acc< 2.6\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_array_get_timestamp((*kml).locations, 1i32 as size_t)
        == 1544739072i32 as libc::c_long) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            216i32,
            b"dc_array_get_timestamp(kml->locations, 1)==1544739072\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    dc_kml_unref(kml);
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
            )) as libc::c_int as libc::c_long
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
            )) as libc::c_int as libc::c_long
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
        ) == 7i32 as libc::c_ulonglong) as libc::c_int as libc::c_long
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
        let mut abs_path: *mut libc::c_char = dc_mprintf(
            b"%s/%s\x00" as *const u8 as *const libc::c_char,
            (*context).blobdir,
            b"foobar\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != (0 == dc_is_blobdir_path(context, abs_path)) as libc::c_int as libc::c_long {
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
            )) as libc::c_int as libc::c_long
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
            )) as libc::c_int as libc::c_long
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
        if 0 != (0 == dc_file_exist(context, abs_path)) as libc::c_int as libc::c_long {
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
            )) as libc::c_int as libc::c_long
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
        ) == 7i32 as libc::c_ulonglong) as libc::c_int as libc::c_long
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
            )) as libc::c_int as libc::c_long
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
            )) as libc::c_int as libc::c_long
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
            )) as libc::c_int as libc::c_long
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
            )) as libc::c_int as libc::c_long
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
            )) as libc::c_int as libc::c_long
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
        let mut fn0: *mut libc::c_char = dc_get_fine_pathNfilename(
            context,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            b"foobar.dadada\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != fn0.is_null() as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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
        let mut fn1: *mut libc::c_char = dc_get_fine_pathNfilename(
            context,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            b"foobar.dadada\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != fn1.is_null() as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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
        if 0 != (0 == dc_delete_file(context, fn0)) as libc::c_int as libc::c_long {
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
    let mut txt: *const libc::c_char =
        b"FieldA: ValueA\nFieldB: ValueB\n\x00" as *const u8 as *const libc::c_char;
    let mut mime: *mut mailmime = 0 as *mut mailmime;
    let mut dummy: size_t = 0i32 as size_t;
    if 0 != !(mailmime_parse(txt, strlen(txt), &mut dummy, &mut mime)
        == MAIL_NO_ERROR as libc::c_int) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            286i32,
            b"mailmime_parse(txt, strlen(txt), &dummy, &mime) == MAIL_NO_ERROR\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != mime.is_null() as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            287i32,
            b"mime != NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut fields: *mut mailimf_fields = mailmime_find_mailimf_fields(mime);
    if 0 != fields.is_null() as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            290i32,
            b"fields != NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut of_a: *mut mailimf_optional_field =
        mailimf_find_optional_field(fields, b"fielda\x00" as *const u8 as *const libc::c_char);
    if 0 != !(!of_a.is_null() && !(*of_a).fld_value.is_null()) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            293i32,
            b"of_a && of_a->fld_value\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        (*of_a).fld_name,
        b"FieldA\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            294i32,
            b"strcmp(of_a->fld_name, \"FieldA\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        (*of_a).fld_value,
        b"ValueA\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            295i32,
            b"strcmp(of_a->fld_value, \"ValueA\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    of_a = mailimf_find_optional_field(fields, b"FIELDA\x00" as *const u8 as *const libc::c_char);
    if 0 != !(!of_a.is_null() && !(*of_a).fld_value.is_null()) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            298i32,
            b"of_a && of_a->fld_value\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        (*of_a).fld_name,
        b"FieldA\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            299i32,
            b"strcmp(of_a->fld_name, \"FieldA\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        (*of_a).fld_value,
        b"ValueA\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            300i32,
            b"strcmp(of_a->fld_value, \"ValueA\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut of_b: *mut mailimf_optional_field =
        mailimf_find_optional_field(fields, b"FieldB\x00" as *const u8 as *const libc::c_char);
    if 0 != !(!of_b.is_null() && !(*of_b).fld_value.is_null()) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            303i32,
            b"of_b && of_b->fld_value\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        (*of_b).fld_value,
        b"ValueB\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            304i32,
            b"strcmp(of_b->fld_value, \"ValueB\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    mailmime_free(mime);
    let mut mimeparser: *mut dc_mimeparser_t = dc_mimeparser_new((*context).blobdir, context);
    let mut raw: *const libc::c_char =
        b"Content-Type: multipart/mixed; boundary=\"==break==\";\nSubject: outer-subject\nX-Special-A: special-a\nFoo: Bar\nChat-Version: 0.0\n\n--==break==\nContent-Type: text/plain; protected-headers=\"v1\";\nSubject: inner-subject\nX-Special-B: special-b\nFoo: Xy\nChat-Version: 1.0\n\ntest1\n\n--==break==--\n\n\x00"
            as *const u8 as *const libc::c_char;
    dc_mimeparser_parse(mimeparser, raw, strlen(raw));
    if 0 != !(strcmp(
        (*mimeparser).subject,
        b"inner-subject\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            336i32,
            b"strcmp(mimeparser->subject, \"inner-subject\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    let mut of: *mut mailimf_optional_field = dc_mimeparser_lookup_optional_field(
        mimeparser,
        b"X-Special-A\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(
        (*of).fld_value,
        b"special-a\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            339i32,
            b"strcmp(of->fld_value, \"special-a\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    of = dc_mimeparser_lookup_optional_field(
        mimeparser,
        b"Foo\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(
        (*of).fld_value,
        b"Bar\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            342i32,
            b"strcmp(of->fld_value, \"Bar\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    of = dc_mimeparser_lookup_optional_field(
        mimeparser,
        b"Chat-Version\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(
        (*of).fld_value,
        b"1.0\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            345i32,
            b"strcmp(of->fld_value, \"1.0\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(carray_count((*mimeparser).parts) == 1i32 as libc::c_uint) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            347i32,
            b"carray_count(mimeparser->parts) == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_mimeparser_unref(mimeparser);
    let mut type_0: libc::c_int = 0;
    let mut mime_0: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_msg_guess_msgtype_from_suffix(
        b"foo/bar-sth.mp3\x00" as *const u8 as *const libc::c_char,
        0 as *mut libc::c_int,
        0 as *mut *mut libc::c_char,
    );
    dc_msg_guess_msgtype_from_suffix(
        b"foo/bar-sth.mp3\x00" as *const u8 as *const libc::c_char,
        0 as *mut libc::c_int,
        &mut mime_0,
    );
    if 0 != !(strcmp(
        mime_0,
        b"audio/mpeg\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            360i32,
            b"strcmp(mime, \"audio/mpeg\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_msg_guess_msgtype_from_suffix(
        b"foo/bar-sth.mp3\x00" as *const u8 as *const libc::c_char,
        &mut type_0,
        0 as *mut *mut libc::c_char,
    );
    if 0 != !(type_0 == 40i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            362i32,
            b"type == DC_MSG_AUDIO\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(mime_0 as *mut libc::c_void);
    if 0 != !(atol(b"\x00" as *const u8 as *const libc::c_char) == 0i32 as libc::c_long)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            370i32,
            b"atol(\"\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(atoi(b"\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            371i32,
            b"atoi(\"\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut f: libc::c_double = dc_atof(b"1.23\x00" as *const u8 as *const libc::c_char);
    if 0 != !(f > 1.22f64 && f < 1.24f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            374i32,
            b"f>1.22 && f<1.24\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut s: *mut libc::c_char = dc_ftoa(1.23f64);
    if 0 != !(dc_atof(s) > 1.22f64 && dc_atof(s) < 1.24f64) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            377i32,
            b"dc_atof(s)>1.22 && dc_atof(s)<1.24\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(s as *mut libc::c_void);
    if 0 != (0 != dc_may_be_valid_addr(0 as *const libc::c_char)) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            380i32,
            b"!dc_may_be_valid_addr(NULL)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"\x00" as *const u8 as *const libc::c_char)) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            381i32,
            b"!dc_may_be_valid_addr(\"\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 == dc_may_be_valid_addr(b"user@domain.tld\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            382i32,
            b"dc_may_be_valid_addr(\"user@domain.tld\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"uuu\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            383i32,
            b"!dc_may_be_valid_addr(\"uuu\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"dd.tt\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            384i32,
            b"!dc_may_be_valid_addr(\"dd.tt\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"tt.dd@uu\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            385i32,
            b"!dc_may_be_valid_addr(\"tt.dd@uu\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"uu\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            386i32,
            b"!dc_may_be_valid_addr(\"uu\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"u@d\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            387i32,
            b"!dc_may_be_valid_addr(\"u@d\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"u@d.\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            388i32,
            b"!dc_may_be_valid_addr(\"u@d.\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"u@d.t\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            389i32,
            b"!dc_may_be_valid_addr(\"u@d.t\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 == dc_may_be_valid_addr(b"u@d.tt\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            390i32,
            b"dc_may_be_valid_addr(\"u@d.tt\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"u@.tt\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            391i32,
            b"!dc_may_be_valid_addr(\"u@.tt\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_may_be_valid_addr(b"@d.tt\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            392i32,
            b"!dc_may_be_valid_addr(\"@d.tt\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut str: *mut libc::c_char = strdup(b"aaa\x00" as *const u8 as *const libc::c_char);
    let mut replacements: libc::c_int = dc_str_replace(
        &mut str,
        b"a\x00" as *const u8 as *const libc::c_char,
        b"ab\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(str, b"ababab\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            396i32,
            b"strcmp(str, \"ababab\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(replacements == 3i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            397i32,
            b"replacements == 3\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = strdup(b"this is a little test string\x00" as *const u8 as *const libc::c_char);
    dc_truncate_str(str, 16i32);
    if 0 != !(strcmp(
        str,
        b"this is a [...]\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            402i32,
            b"strcmp(str, \"this is a \" DC_EDITORIAL_ELLIPSE)==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = strdup(b"1234\x00" as *const u8 as *const libc::c_char);
    dc_truncate_str(str, 2i32);
    if 0 != !(strcmp(str, b"1234\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            407i32,
            b"strcmp(str, \"1234\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = strdup(b"1234567\x00" as *const u8 as *const libc::c_char);
    dc_truncate_str(str, 1i32);
    if 0 != !(strcmp(str, b"1[...]\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            412i32,
            b"strcmp(str, \"1[...]\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = strdup(b"123456\x00" as *const u8 as *const libc::c_char);
    dc_truncate_str(str, 4i32);
    if 0 != !(strcmp(str, b"123456\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            417i32,
            b"strcmp(str, \"123456\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = dc_insert_breaks(
        b"just1234test\x00" as *const u8 as *const libc::c_char,
        4i32,
        b" \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(
        str,
        b"just 1234 test\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            421i32,
            b"strcmp(str, \"just 1234 test\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = dc_insert_breaks(
        b"just1234tes\x00" as *const u8 as *const libc::c_char,
        4i32,
        b"--\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(
        str,
        b"just--1234--tes\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            425i32,
            b"strcmp(str, \"just--1234--tes\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = dc_insert_breaks(
        b"just1234t\x00" as *const u8 as *const libc::c_char,
        4i32,
        b"\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(str, b"just1234t\x00" as *const u8 as *const libc::c_char) == 0i32)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            429i32,
            b"strcmp(str, \"just1234t\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = dc_insert_breaks(
        b"\x00" as *const u8 as *const libc::c_char,
        4i32,
        b"---\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(str, b"\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            433i32,
            b"strcmp(str, \"\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = dc_null_terminate(b"abcxyz\x00" as *const u8 as *const libc::c_char, 3i32);
    if 0 != !(strcmp(str, b"abc\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            437i32,
            b"strcmp(str, \"abc\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = dc_null_terminate(b"abcxyz\x00" as *const u8 as *const libc::c_char, 0i32);
    if 0 != !(strcmp(str, b"\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            441i32,
            b"strcmp(str, \"\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    str = dc_null_terminate(0 as *const libc::c_char, 0i32);
    if 0 != !(strcmp(str, b"\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            445i32,
            b"strcmp(str, \"\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str as *mut libc::c_void);
    let mut list: *mut clist = dc_str_to_clist(
        0 as *const libc::c_char,
        b" \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !((*list).count == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            449i32,
            b"clist_count(list)==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    clist_free_content(list);
    clist_free(list);
    list = dc_str_to_clist(
        b"\x00" as *const u8 as *const libc::c_char,
        b" \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !((*list).count == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            454i32,
            b"clist_count(list)==1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    clist_free_content(list);
    clist_free(list);
    list = dc_str_to_clist(
        b" \x00" as *const u8 as *const libc::c_char,
        b" \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !((*list).count == 2i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            459i32,
            b"clist_count(list)==2\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    clist_free_content(list);
    clist_free(list);
    list = dc_str_to_clist(
        b"foo bar test\x00" as *const u8 as *const libc::c_char,
        b" \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !((*list).count == 3i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            464i32,
            b"clist_count(list)==3\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    str = dc_str_from_clist(list, b" \x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(str, b"foo bar test\x00" as *const u8 as *const libc::c_char) == 0i32)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            466i32,
            b"strcmp(str, \"foo bar test\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    clist_free_content(list);
    clist_free(list);
    free(str as *mut libc::c_void);
    if 0 != !(strcmp(
        b"fresh=10\x00" as *const u8 as *const libc::c_char,
        b"fresh=10\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            471i32,
            b"strcmp(\"fresh=\" DC_STRINGIFY(DC_STATE_IN_FRESH), \"fresh=10\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"noticed=13\x00" as *const u8 as *const libc::c_char,
        b"noticed=13\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            472i32,
            b"strcmp(\"noticed=\" DC_STRINGIFY(DC_STATE_IN_NOTICED), \"noticed=13\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"seen=16\x00" as *const u8 as *const libc::c_char,
        b"seen=16\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            473i32,
            b"strcmp(\"seen=\" DC_STRINGIFY(DC_STATE_IN_SEEN), \"seen=16\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"pending=20\x00" as *const u8 as *const libc::c_char,
        b"pending=20\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            474i32,
            b"strcmp(\"pending=\" DC_STRINGIFY(DC_STATE_OUT_PENDING), \"pending=20\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"failed=24\x00" as *const u8 as *const libc::c_char,
        b"failed=24\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            475i32,
            b"strcmp(\"failed=\" DC_STRINGIFY(DC_STATE_OUT_FAILED), \"failed=24\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"delivered=26\x00" as *const u8 as *const libc::c_char,
        b"delivered=26\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            476i32,
            b"strcmp(\"delivered=\" DC_STRINGIFY(DC_STATE_OUT_DELIVERED), \"delivered=26\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"mdn_rcvd=28\x00" as *const u8 as *const libc::c_char,
        b"mdn_rcvd=28\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            477i32,
            b"strcmp(\"mdn_rcvd=\" DC_STRINGIFY(DC_STATE_OUT_MDN_RCVD), \"mdn_rcvd=28\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"undefined=0\x00" as *const u8 as *const libc::c_char,
        b"undefined=0\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            479i32,
            b"strcmp(\"undefined=\" DC_STRINGIFY(DC_CHAT_TYPE_UNDEFINED), \"undefined=0\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"single=100\x00" as *const u8 as *const libc::c_char,
        b"single=100\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            480i32,
            b"strcmp(\"single=\" DC_STRINGIFY(DC_CHAT_TYPE_SINGLE), \"single=100\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"group=120\x00" as *const u8 as *const libc::c_char,
        b"group=120\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            481i32,
            b"strcmp(\"group=\" DC_STRINGIFY(DC_CHAT_TYPE_GROUP), \"group=120\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"vgroup=130\x00" as *const u8 as *const libc::c_char,
        b"vgroup=130\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            482i32,
            b"strcmp(\"vgroup=\" DC_STRINGIFY(DC_CHAT_TYPE_VERIFIED_GROUP), \"vgroup=130\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"deaddrop=1\x00" as *const u8 as *const libc::c_char,
        b"deaddrop=1\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            484i32,
            b"strcmp(\"deaddrop=\" DC_STRINGIFY(DC_CHAT_ID_DEADDROP), \"deaddrop=1\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"trash=3\x00" as *const u8 as *const libc::c_char,
        b"trash=3\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            485i32,
            b"strcmp(\"trash=\" DC_STRINGIFY(DC_CHAT_ID_TRASH), \"trash=3\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"in_creation=4\x00" as *const u8 as *const libc::c_char,
        b"in_creation=4\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 486i32,
                     b"strcmp(\"in_creation=\" DC_STRINGIFY(DC_CHAT_ID_MSGS_IN_CREATION), \"in_creation=4\")==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !(strcmp(
        b"starred=5\x00" as *const u8 as *const libc::c_char,
        b"starred=5\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            487i32,
            b"strcmp(\"starred=\" DC_STRINGIFY(DC_CHAT_ID_STARRED), \"starred=5\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"archivedlink=6\x00" as *const u8 as *const libc::c_char,
        b"archivedlink=6\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 488i32,
                     b"strcmp(\"archivedlink=\" DC_STRINGIFY(DC_CHAT_ID_ARCHIVED_LINK), \"archivedlink=6\")==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !(strcmp(
        b"spcl_chat=9\x00" as *const u8 as *const libc::c_char,
        b"spcl_chat=9\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            489i32,
            b"strcmp(\"spcl_chat=\" DC_STRINGIFY(DC_CHAT_ID_LAST_SPECIAL), \"spcl_chat=9\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"self=1\x00" as *const u8 as *const libc::c_char,
        b"self=1\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            491i32,
            b"strcmp(\"self=\" DC_STRINGIFY(DC_CONTACT_ID_SELF), \"self=1\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"spcl_contact=9\x00" as *const u8 as *const libc::c_char,
        b"spcl_contact=9\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 492i32,
                     b"strcmp(\"spcl_contact=\" DC_STRINGIFY(DC_CONTACT_ID_LAST_SPECIAL), \"spcl_contact=9\")==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !(strcmp(
        b"grpimg=3\x00" as *const u8 as *const libc::c_char,
        b"grpimg=3\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            494i32,
            b"strcmp(\"grpimg=\" DC_STRINGIFY(DC_CMD_GROUPIMAGE_CHANGED), \"grpimg=3\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"notverified=0\x00" as *const u8 as *const libc::c_char,
        b"notverified=0\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            496i32,
            b"strcmp(\"notverified=\" DC_STRINGIFY(DC_NOT_VERIFIED), \"notverified=0\")==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"bidirectional=2\x00" as *const u8 as *const libc::c_char,
        b"bidirectional=2\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 497i32,
                     b"strcmp(\"bidirectional=\" DC_STRINGIFY(DC_BIDIRECT_VERIFIED), \"bidirectional=2\")==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !(strcmp(
        b"public=0\x00" as *const u8 as *const libc::c_char,
        b"public=0\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            499i32,
            b"strcmp(\"public=\" DC_STRINGIFY(DC_KEY_PUBLIC), \"public=0\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"private=1\x00" as *const u8 as *const libc::c_char,
        b"private=1\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            500i32,
            b"strcmp(\"private=\" DC_STRINGIFY(DC_KEY_PRIVATE), \"private=1\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !('f' as i32 == 'f' as i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            502i32,
            b"DC_PARAM_FILE == \'f\'\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !('w' as i32 == 'w' as i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            503i32,
            b"DC_PARAM_WIDTH == \'w\'\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !('h' as i32 == 'h' as i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            504i32,
            b"DC_PARAM_HEIGHT == \'h\'\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !('d' as i32 == 'd' as i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            505i32,
            b"DC_PARAM_DURATION == \'d\'\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !('m' as i32 == 'm' as i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            506i32,
            b"DC_PARAM_MIMETYPE == \'m\'\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !('a' as i32 == 'a' as i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            507i32,
            b"DC_PARAM_FORWARDED == \'a\'\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !('U' as i32 == 'U' as i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            508i32,
            b"DC_PARAM_UNPROMOTED == \'U\'\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut buf1: *mut libc::c_char =
        strdup(b"ol\xc3\xa1 mundo <>\"\'& \xc3\xa4\xc3\x84\xc3\xb6\xc3\x96\xc3\xbc\xc3\x9c\xc3\x9f foo\xc3\x86\xc3\xa7\xc3\x87 \xe2\x99\xa6&noent;\x00"
                   as *const u8 as *const libc::c_char);
    let mut buf2: *mut libc::c_char = strdup(buf1);
    dc_replace_bad_utf8_chars(buf2);
    if 0 != !(strcmp(buf1, buf2) == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            512i32,
            b"strcmp(buf1, buf2)==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    buf1 = strdup(b"ISO-String with Ae: \xc4\x00" as *const u8 as *const libc::c_char);
    buf2 = strdup(buf1);
    dc_replace_bad_utf8_chars(buf2);
    if 0 != !(strcmp(
        b"ISO-String with Ae: _\x00" as *const u8 as *const libc::c_char,
        buf2,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            517i32,
            b"strcmp(\"ISO-String with Ae: _\", buf2)==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    buf1 = strdup(b"\x00" as *const u8 as *const libc::c_char);
    buf2 = strdup(buf1);
    dc_replace_bad_utf8_chars(buf2);
    if 0 != !(*buf2.offset(0isize) as libc::c_int == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            522i32,
            b"buf2[0]==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    dc_replace_bad_utf8_chars(0 as *mut libc::c_char);
    buf1 = dc_urlencode(b"Bj\xc3\xb6rn Petersen\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(
        buf1,
        b"Bj%C3%B6rn+Petersen\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            528i32,
            b"strcmp(buf1, \"Bj%C3%B6rn+Petersen\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    buf2 = dc_urldecode(buf1);
    if 0 != !(strcmp(
        buf2,
        b"Bj\xc3\xb6rn Petersen\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            530i32,
            b"strcmp(buf2, \"Bj\xc3\xb6rn Petersen\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    buf1 = dc_create_id();
    if 0 != !(strlen(buf1) == 11) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            534i32,
            b"strlen(buf1) == DC_CREATE_ID_LEN\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_decode_header_words(
        b"=?utf-8?B?dGVzdMOkw7bDvC50eHQ=?=\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(
        buf1,
        b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            538i32,
            b"strcmp(buf1, \"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_decode_header_words(b"just ascii test\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(
        buf1,
        b"just ascii test\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            542i32,
            b"strcmp(buf1, \"just ascii test\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_encode_header_words(b"abcdef\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(buf1, b"abcdef\x00" as *const u8 as *const libc::c_char) == 0i32)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            546i32,
            b"strcmp(buf1, \"abcdef\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_encode_header_words(
        b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strncmp(buf1, b"=?utf-8\x00" as *const u8 as *const libc::c_char, 7) == 0i32)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            550i32,
            b"strncmp(buf1, \"=?utf-8\", 7)==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    buf2 = dc_decode_header_words(buf1);
    if 0 != !(strcmp(
        b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\x00" as *const u8 as *const libc::c_char,
        buf2,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            552i32,
            b"strcmp(\"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\", buf2)==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    buf1 =
        dc_decode_header_words(b"=?ISO-8859-1?Q?attachment=3B=0D=0A_filename=3D?= =?ISO-8859-1?Q?=22test=E4=F6=FC=2Etxt=22=3B=0D=0A_size=3D39?=\x00"
                                   as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(
        buf1,
        b"attachment;\r\n filename=\"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\";\r\n size=39\x00"
            as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 557i32,
                     b"strcmp(buf1, \"attachment;\\r\\n filename=\\\"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\\\";\\r\\n size=39\")==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_encode_ext_header(b"Bj\xc3\xb6rn Petersen\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(
        buf1,
        b"utf-8\'\'Bj%C3%B6rn%20Petersen\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            561i32,
            b"strcmp(buf1, \"utf-8\'\'Bj%C3%B6rn%20Petersen\") == 0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    buf2 = dc_decode_ext_header(buf1);
    if 0 != !(strcmp(
        buf2,
        b"Bj\xc3\xb6rn Petersen\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            563i32,
            b"strcmp(buf2, \"Bj\xc3\xb6rn Petersen\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    buf1 = dc_decode_ext_header(
        b"iso-8859-1\'en\'%A3%20rates\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(strcmp(
        buf1,
        b"\xc2\xa3 rates\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            568i32,
            b"strcmp(buf1, \"\xc2\xa3 rates\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        buf1,
        b"\xc2\xa3 rates\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            569i32,
            b"strcmp(buf1, \"\\xC2\\xA3 rates\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_decode_ext_header(b"wrong\'format\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(
        buf1,
        b"wrong\'format\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            573i32,
            b"strcmp(buf1, \"wrong\'format\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_decode_ext_header(b"\'\'\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(buf1, b"\'\'\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            577i32,
            b"strcmp(buf1, \"\'\'\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_decode_ext_header(b"x\'\'\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(buf1, b"\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            581i32,
            b"strcmp(buf1, \"\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_decode_ext_header(b"\'\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(buf1, b"\'\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            585i32,
            b"strcmp(buf1, \"\'\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    buf1 = dc_decode_ext_header(b"\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(buf1, b"\x00" as *const u8 as *const libc::c_char) == 0i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            589i32,
            b"strcmp(buf1, \"\") == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    if 0 != (0 == dc_needs_ext_header(b"Bj\xc3\xb6rn\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            592i32,
            b"dc_needs_ext_header(\"Bj\xc3\xb6rn\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_needs_ext_header(b"Bjoern\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            593i32,
            b"!dc_needs_ext_header(\"Bjoern\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_needs_ext_header(b"\x00" as *const u8 as *const libc::c_char)) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            594i32,
            b"!dc_needs_ext_header(\"\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 == dc_needs_ext_header(b" \x00" as *const u8 as *const libc::c_char)) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            595i32,
            b"dc_needs_ext_header(\" \")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 == dc_needs_ext_header(b"a b\x00" as *const u8 as *const libc::c_char))
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            596i32,
            b"dc_needs_ext_header(\"a b\")\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 != dc_needs_ext_header(0 as *const libc::c_char)) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            597i32,
            b"!dc_needs_ext_header(NULL)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    buf1 = dc_encode_modified_utf7(
        b"Bj\xc3\xb6rn Petersen\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    if 0 != !(strcmp(
        buf1,
        b"Bj&APY-rn_Petersen\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            600i32,
            b"strcmp(buf1, \"Bj&APY-rn_Petersen\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    buf2 = dc_decode_modified_utf7(buf1, 1i32);
    if 0 != !(strcmp(
        buf2,
        b"Bj\xc3\xb6rn Petersen\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            602i32,
            b"strcmp(buf2, \"Bj\xc3\xb6rn Petersen\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(buf1 as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    if 0 != !(2100i32 == 2100i32 || 2100i32 == 2052i32 || 2100i32 == 2055i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            606i32,
            b"DC_EVENT_DATA1_IS_STRING(2100)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(2052i32 == 2100i32 || 2052i32 == 2052i32 || 2052i32 == 2055i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            607i32,
            b"DC_EVENT_DATA1_IS_STRING(2052)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (100i32 == 2100i32 || 100i32 == 2052i32 || 100i32 == 2055i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            608i32,
            b"!DC_EVENT_DATA1_IS_STRING(100)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (300i32 == 2100i32 || 300i32 == 2052i32 || 300i32 == 2055i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            609i32,
            b"!DC_EVENT_DATA1_IS_STRING(300)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (400i32 == 2100i32 || 400i32 == 2052i32 || 400i32 == 2055i32) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            610i32,
            b"!DC_EVENT_DATA1_IS_STRING(400)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(100i32 >= 100i32 && 100i32 <= 499i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            612i32,
            b"DC_EVENT_DATA2_IS_STRING(100)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(300i32 >= 100i32 && 300i32 <= 499i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            613i32,
            b"DC_EVENT_DATA2_IS_STRING(300)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(400i32 >= 100i32 && 400i32 <= 499i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            614i32,
            b"DC_EVENT_DATA2_IS_STRING(400)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (2010i32 >= 100i32 && 2010i32 <= 499i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            615i32,
            b"!DC_EVENT_DATA2_IS_STRING(2010)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(2091i32 == 2091i32 || 2091i32 == 2100i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            617i32,
            b"DC_EVENT_RETURNS_STRING(2091)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(2100i32 == 2091i32 || 2100i32 == 2100i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            618i32,
            b"DC_EVENT_RETURNS_STRING(2100)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (100i32 == 2091i32 || 100i32 == 2100i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            619i32,
            b"!DC_EVENT_RETURNS_STRING(100)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (300i32 == 2091i32 || 300i32 == 2100i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            620i32,
            b"!DC_EVENT_RETURNS_STRING(300)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (400i32 == 2091i32 || 400i32 == 2100i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            621i32,
            b"!DC_EVENT_RETURNS_STRING(400)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_utf8_strlen(b"c\x00" as *const u8 as *const libc::c_char) == 1
        && strlen(b"c\x00" as *const u8 as *const libc::c_char) == 1) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            623i32,
            b"dc_utf8_strlen(\"c\")==1 && strlen(\"c\")==1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_utf8_strlen(b"\xc3\xa4\x00" as *const u8 as *const libc::c_char) == 1
        && strlen(b"\xc3\xa4\x00" as *const u8 as *const libc::c_char) == 2)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            624i32,
            b"dc_utf8_strlen(\"\xc3\xa4\")==1 && strlen(\"\xc3\xa4\")==2\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    let mut arr = dc_array_new(7i32 as size_t);
    if 0 != !(dc_array_get_cnt(arr) == 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            633i32,
            b"dc_array_get_cnt(arr) == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut i: libc::c_int = 0;
    i = 0i32;
    while i < 1000i32 {
        dc_array_add_id(arr, (i + 1i32 * 2i32) as uint32_t);
        i += 1
    }
    if 0 != !(dc_array_get_cnt(arr) == 1000) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            639i32,
            b"dc_array_get_cnt(arr) == TEST_CNT\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    i = 0i32;
    while i < 1000i32 {
        if 0 != !(dc_array_get_id(arr, i as size_t) == (i + 1i32 * 2i32) as libc::c_uint)
            as libc::c_int as libc::c_long
        {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                642i32,
                b"dc_array_get_id(arr, i) == i+1*2\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        i += 1
    }
    if 0 != !(dc_array_get_id(arr, -1i32 as size_t) == 0i32 as libc::c_uint) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            644i32,
            b"dc_array_get_id(arr, -1) == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_array_get_id(arr, 1000i32 as size_t) == 0i32 as libc::c_uint) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            645i32,
            b"dc_array_get_id(arr, TEST_CNT) == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_array_get_id(arr, (1000i32 + 1i32) as size_t) == 0i32 as libc::c_uint)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            646i32,
            b"dc_array_get_id(arr, TEST_CNT+1) == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_array_empty(arr);
    if 0 != !(dc_array_get_cnt(arr) == 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            649i32,
            b"dc_array_get_cnt(arr) == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_array_add_id(arr, 13i32 as uint32_t);
    dc_array_add_id(arr, 7i32 as uint32_t);
    dc_array_add_id(arr, 666i32 as uint32_t);
    dc_array_add_id(arr, 0i32 as uint32_t);
    dc_array_add_id(arr, 5000i32 as uint32_t);
    dc_array_sort_ids(arr);
    if 0 != !(dc_array_get_id(arr, 0i32 as size_t) == 0i32 as libc::c_uint
        && dc_array_get_id(arr, 1i32 as size_t) == 7i32 as libc::c_uint
        && dc_array_get_id(arr, 2i32 as size_t) == 13i32 as libc::c_uint
        && dc_array_get_id(arr, 3i32 as size_t) == 666i32 as libc::c_uint)
        as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 657i32,
                     b"dc_array_get_id(arr, 0)==0 && dc_array_get_id(arr, 1)==7 && dc_array_get_id(arr, 2)==13 && dc_array_get_id(arr, 3)==666\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    let mut str_0: *mut libc::c_char =
        dc_array_get_string(arr, b"-\x00" as *const u8 as *const libc::c_char);
    if 0 != !(strcmp(
        str_0,
        b"0-7-13-666-5000\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            660i32,
            b"strcmp(str, \"0-7-13-666-5000\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str_0 as *mut libc::c_void);
    let arr2: [uint32_t; 4] = [
        0i32 as uint32_t,
        12i32 as uint32_t,
        133i32 as uint32_t,
        1999999i32 as uint32_t,
    ];
    str_0 = dc_arr_to_string(arr2.as_ptr(), 4i32);
    if 0 != !(strcmp(
        str_0,
        b"0,12,133,1999999\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            665i32,
            b"strcmp(str, \"0,12,133,1999999\")==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(str_0 as *mut libc::c_void);
    dc_array_empty(arr);
    dc_array_add_ptr(
        arr,
        b"XX\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
    );
    dc_array_add_ptr(
        arr,
        b"item1\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
    );
    dc_array_add_ptr(
        arr,
        b"bbb\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
    );
    dc_array_add_ptr(
        arr,
        b"aaa\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
    );
    dc_array_sort_strings(arr);
    if 0 != !(strcmp(
        b"XX\x00" as *const u8 as *const libc::c_char,
        dc_array_get_ptr(arr, 0i32 as size_t) as *mut libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            674i32,
            b"strcmp(\"XX\", (char*)dc_array_get_ptr(arr, 0))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"aaa\x00" as *const u8 as *const libc::c_char,
        dc_array_get_ptr(arr, 1i32 as size_t) as *mut libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            675i32,
            b"strcmp(\"aaa\", (char*)dc_array_get_ptr(arr, 1))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"bbb\x00" as *const u8 as *const libc::c_char,
        dc_array_get_ptr(arr, 2i32 as size_t) as *mut libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            676i32,
            b"strcmp(\"bbb\", (char*)dc_array_get_ptr(arr, 2))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        b"item1\x00" as *const u8 as *const libc::c_char,
        dc_array_get_ptr(arr, 3i32 as size_t) as *mut libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            677i32,
            b"strcmp(\"item1\", (char*)dc_array_get_ptr(arr, 3))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    dc_array_unref(arr);
    let mut p1: *mut dc_param_t = dc_param_new();
    dc_param_set_packed(
        p1,
        b"\r\n\r\na=1\nb=2\n\nc = 3 \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(dc_param_get_int(p1, 'a' as i32, 0i32) == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            690i32,
            b"dc_param_get_int(p1, \'a\', 0)==1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_param_get_int(p1, 'b' as i32, 0i32) == 2i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            691i32,
            b"dc_param_get_int(p1, \'b\', 0)==2\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_param_get_int(p1, 'c' as i32, 0i32) == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            692i32,
            b"dc_param_get_int(p1, \'c\', 0)==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(dc_param_exists(p1, 'c' as i32) == 0i32) as libc::c_int as libc::c_long {
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
    if 0 != !(dc_param_get_int(p1, 'd' as i32, 0i32) == 4i32) as libc::c_int as libc::c_long {
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
    ) == 0i32) as libc::c_int as libc::c_long
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
    if 0 != !(!keys.is_null() && 0 != *keys.offset(0isize) as libc::c_int) as libc::c_int
        as libc::c_long
    {
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
    .is_null() as libc::c_int as libc::c_long
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
    if 0 != strstr(keys, b" addr \x00" as *const u8 as *const libc::c_char).is_null() as libc::c_int
        as libc::c_long
    {
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
    .is_null() as libc::c_int as libc::c_long
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
    if 0 != strstr(keys, b" mail_user \x00" as *const u8 as *const libc::c_char).is_null()
        as libc::c_int as libc::c_long
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
    if 0 != strstr(keys, b" mail_pw \x00" as *const u8 as *const libc::c_char).is_null()
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            732i32,
            b"strstr(keys, \" mail_pw \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" mail_port \x00" as *const u8 as *const libc::c_char).is_null()
        as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    if 0 != strstr(keys, b" send_user \x00" as *const u8 as *const libc::c_char).is_null()
        as libc::c_int as libc::c_long
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
    if 0 != strstr(keys, b" send_pw \x00" as *const u8 as *const libc::c_char).is_null()
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            736i32,
            b"strstr(keys, \" send_pw \")!=NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != strstr(keys, b" send_port \x00" as *const u8 as *const libc::c_char).is_null()
        as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    .is_null() as libc::c_int as libc::c_long
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
    let mut ah: *mut dc_aheader_t = dc_aheader_new();
    let mut rendered: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ah_ok: libc::c_int = 0;
    ah_ok = dc_aheader_set_from_string(
        ah,
        b"addr=a@b.example.org; prefer-encrypt=mutual; keydata=RGVsdGEgQ2hhdA==\x00" as *const u8
            as *const libc::c_char,
    );
    if 0 != !(ah_ok == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            769i32,
            b"ah_ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!(*ah).addr.is_null()
        && strcmp(
            (*ah).addr,
            b"a@b.example.org\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            770i32,
            b"ah->addr && strcmp(ah->addr, \"a@b.example.org\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !((*(*ah).public_key).bytes == 10i32
        && strncmp(
            (*(*ah).public_key).binary as *mut libc::c_char,
            b"Delta Chat\x00" as *const u8 as *const libc::c_char,
            10,
        ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 771i32,
                     b"ah->public_key->bytes==10 && strncmp((char*)ah->public_key->binary, \"Delta Chat\", 10)==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !((*ah).prefer_encrypt == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            772i32,
            b"ah->prefer_encrypt==DC_PE_MUTUAL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    rendered = dc_aheader_render(ah);
    if 0 != !(!rendered.is_null()
        && strcmp(
            rendered,
            b"addr=a@b.example.org; prefer-encrypt=mutual; keydata= RGVsdGEgQ2hhdA==\x00"
                as *const u8 as *const libc::c_char,
        ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 775i32,
                     b"rendered && strcmp(rendered, \"addr=a@b.example.org; prefer-encrypt=mutual; keydata= RGVsdGEgQ2hhdA==\")==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    ah_ok =
        dc_aheader_set_from_string(ah,
                                   b" _foo; __FOO=BAR ;;; addr = a@b.example.org ;\r\n   prefer-encrypt = mutual ; keydata = RG VsdGEgQ\r\n2hhdA==\x00"
                                       as *const u8 as *const libc::c_char);
    if 0 != !(ah_ok == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            778i32,
            b"ah_ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(!(*ah).addr.is_null()
        && strcmp(
            (*ah).addr,
            b"a@b.example.org\x00" as *const u8 as *const libc::c_char,
        ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            779i32,
            b"ah->addr && strcmp(ah->addr, \"a@b.example.org\")==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !((*(*ah).public_key).bytes == 10i32
        && strncmp(
            (*(*ah).public_key).binary as *mut libc::c_char,
            b"Delta Chat\x00" as *const u8 as *const libc::c_char,
            10,
        ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 780i32,
                     b"ah->public_key->bytes==10 && strncmp((char*)ah->public_key->binary, \"Delta Chat\", 10)==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !((*ah).prefer_encrypt == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            781i32,
            b"ah->prefer_encrypt==DC_PE_MUTUAL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(
        ah,
        b"addr=a@b.example.org; prefer-encrypt=ignoreUnknownValues; keydata=RGVsdGEgQ2hhdA==\x00"
            as *const u8 as *const libc::c_char,
    );
    if 0 != !(ah_ok == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            784i32,
            b"ah_ok == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(
        ah,
        b"addr=a@b.example.org; keydata=RGVsdGEgQ2hhdA==\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(ah_ok == 1i32 && (*ah).prefer_encrypt == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            787i32,
            b"ah_ok == 1 && ah->prefer_encrypt==DC_PE_NOPREFERENCE\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(ah, b"\x00" as *const u8 as *const libc::c_char);
    if 0 != !(ah_ok == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            790i32,
            b"ah_ok == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(ah, b";\x00" as *const u8 as *const libc::c_char);
    if 0 != !(ah_ok == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            793i32,
            b"ah_ok == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(ah, b"foo\x00" as *const u8 as *const libc::c_char);
    if 0 != !(ah_ok == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            796i32,
            b"ah_ok == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(ah, b"\n\n\n\x00" as *const u8 as *const libc::c_char);
    if 0 != !(ah_ok == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            799i32,
            b"ah_ok == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(ah, b" ;;\x00" as *const u8 as *const libc::c_char);
    if 0 != !(ah_ok == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            802i32,
            b"ah_ok == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    ah_ok = dc_aheader_set_from_string(
        ah,
        b"addr=a@t.de; unknwon=1; keydata=jau\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != !(ah_ok == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            805i32,
            b"ah_ok == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_aheader_unref(ah);
    free(rendered as *mut libc::c_void);
    let mut ok: libc::c_int = 0;
    let mut buf_0: *mut libc::c_char = 0 as *mut libc::c_char;
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
    if 0 != !(ok == 1i32) as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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
    // FIXME
    // if 0 != !(!base64.is_null()
    //     && strcmp(base64, b"data\x00" as *const u8 as *const libc::c_char) == 0i32)
    //     as libc::c_int as libc::c_long
    // {
    //     __assert_rtn(
    //         (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
    //             .as_ptr(),
    //         b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
    //         823i32,
    //         b"base64 && strcmp(base64, \"data\") == 0\x00" as *const u8 as *const libc::c_char,
    //     );
    // } else {
    // };
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
    if 0 != !(ok == 1i32) as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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

    // FIXME
    // if 0 != !(!base64.is_null()
    //     && strcmp(base64, b"dat1\x00" as *const u8 as *const libc::c_char) == 0i32)
    //     as libc::c_int as libc::c_long
    // {
    //     __assert_rtn(
    //         (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
    //             .as_ptr(),
    //         b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
    //         830i32,
    //         b"base64 && strcmp(base64, \"dat1\") == 0\x00" as *const u8 as *const libc::c_char,
    //     );
    // } else {
    // };
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
    if 0 != !(ok == 1i32) as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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
    if 0 != !setupcodebegin.is_null() as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            837i32,
            b"setupcodebegin == NULL\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };

    // FIXME
    // if 0 != !(!base64.is_null()
    //     && strcmp(
    //         base64,
    //         b"base64-123\x00" as *const u8 as *const libc::c_char,
    //     ) == 0i32) as libc::c_int as libc::c_long
    // {
    //     __assert_rtn(
    //         (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
    //             .as_ptr(),
    //         b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
    //         838i32,
    //         b"base64 && strcmp(base64, \"base64-123\")==0\x00" as *const u8 as *const libc::c_char,
    //     );
    // } else {
    // };
    free(buf_0 as *mut libc::c_void);
    buf_0 = strdup(b"foo-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char);
    ok = dc_split_armored_data(
        buf_0,
        &mut headerline,
        &mut setupcodebegin,
        0 as *mut *const libc::c_char,
        &mut base64,
    );
    if 0 != !(ok == 0i32) as libc::c_int as libc::c_long {
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
    if 0 != !(ok == 1i32) as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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
        ) == 0i32) as libc::c_int as libc::c_long
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

    // FIXME
    // if 0 != !(!base64.is_null()
    //     && strcmp(
    //         base64,
    //         b"base64-567 \n abc\x00" as *const u8 as *const libc::c_char,
    //     ) == 0i32) as libc::c_int as libc::c_long
    // {
    //     __assert_rtn(
    //         (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
    //             .as_ptr(),
    //         b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
    //         851i32,
    //         b"base64 && strcmp(base64, \"base64-567 \\n abc\")==0\x00" as *const u8
    //             as *const libc::c_char,
    //     );
    // } else {
    // };
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
    if 0 != !(ok == 1i32) as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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
        ) == 0i32) as libc::c_int as libc::c_long
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

    // FIXME
    // if 0 != !(!base64.is_null()
    //     && strcmp(base64, b"base64\x00" as *const u8 as *const libc::c_char) == 0i32)
    //     as libc::c_int as libc::c_long
    // {
    //     __assert_rtn(
    //         (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
    //             .as_ptr(),
    //         b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
    //         859i32,
    //         b"base64 && strcmp(base64, \"base64\")==0\x00" as *const u8 as *const libc::c_char,
    //     );
    // } else {
    // };
    free(buf_0 as *mut libc::c_void);
    let mut norm: *mut libc::c_char = dc_normalize_setup_code(
        context,
        b"123422343234423452346234723482349234\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != norm.is_null() as libc::c_int as libc::c_long {
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
    ) == 0i32) as libc::c_int as libc::c_long
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
    if 0 != norm.is_null() as libc::c_int as libc::c_long {
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
    ) == 0i32) as libc::c_int as libc::c_long
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
    let mut buf_1: *mut libc::c_char = 0 as *mut libc::c_char;
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
        )) as libc::c_int as libc::c_long
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
        ) == 0i32) as libc::c_int as libc::c_long
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
        as libc::c_int as libc::c_long
    {
        __assert_rtn((*::std::mem::transmute::<&[u8; 17],
                                               &[libc::c_char; 17]>(b"stress_functions\x00")).as_ptr(),
                     b"../cmdline/stress.c\x00" as *const u8 as
                         *const libc::c_char, 885i32,
                     b"setupcodebegin && strlen(setupcodebegin)<strlen(s_em_setupcode) && strncmp(setupcodebegin, s_em_setupcode, strlen(setupcodebegin))==0\x00"
                         as *const u8 as *const libc::c_char);
    } else {
    };
    if 0 != !preferencrypt_0.is_null() as libc::c_int as libc::c_long {
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
    if 0 != buf_1.is_null() as libc::c_int as libc::c_long {
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
        )) as libc::c_int as libc::c_long
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
        ) == 0i32) as libc::c_int as libc::c_long
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
    if 0 != !setupcodebegin_0.is_null() as libc::c_int as libc::c_long {
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
        ) == 0i32) as libc::c_int as libc::c_long
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
        let mut setupcode: *mut libc::c_char = 0 as *mut libc::c_char;
        let mut setupfile: *mut libc::c_char = 0 as *mut libc::c_char;
        setupcode = dc_create_setup_code(context);
        if 0 != setupcode.is_null() as libc::c_int as libc::c_long {
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
        if 0 != !(strlen(setupcode) == 44) as libc::c_int as libc::c_long {
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
            && *setupcode.offset(39isize) as libc::c_int == '-' as i32)
            as libc::c_int as libc::c_long
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
        if 0 != setupfile.is_null() as libc::c_int as libc::c_long {
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
        let mut buf_2: *mut libc::c_char = dc_strdup(setupfile);
        let mut headerline_1: *const libc::c_char = 0 as *const libc::c_char;
        let mut setupcodebegin_1: *const libc::c_char = 0 as *const libc::c_char;
        if 0 != (0
            == dc_split_armored_data(
                buf_2,
                &mut headerline_1,
                &mut setupcodebegin_1,
                0 as *mut *const libc::c_char,
                0 as *mut *const libc::c_char,
            )) as libc::c_int as libc::c_long
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
            ) == 0i32) as libc::c_int as libc::c_long
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
            && strncmp(setupcodebegin_1, setupcode, 2) == 0i32) as libc::c_int
            as libc::c_long
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
        let mut payload: *mut libc::c_char = 0 as *mut libc::c_char;
        let mut headerline_2: *const libc::c_char = 0 as *const libc::c_char;
        payload = dc_decrypt_setup_file(context, setupcode, setupfile);
        if 0 != payload.is_null() as libc::c_int as libc::c_long {
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
            )) as libc::c_int as libc::c_long
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
            ) == 0i32) as libc::c_int as libc::c_long
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
    let mut bad_key: *mut dc_key_t = dc_key_new();
    let mut bad_data: [libc::c_uchar; 4096] = [0; 4096];
    let mut i_0: libc::c_int = 0i32;
    while i_0 < 4096i32 {
        bad_data[i_0 as usize] = (i_0 & 0xffi32) as libc::c_uchar;
        i_0 += 1
    }
    let mut j: libc::c_int = 0i32;
    while j < 4096i32 / 40i32 {
        dc_key_set_from_binary(
            bad_key,
            &mut *bad_data.as_mut_ptr().offset(j as isize) as *mut libc::c_uchar
                as *const libc::c_void,
            4096i32 / 2i32 + j,
            if 0 != j & 1i32 { 0i32 } else { 1i32 },
        );
        if 0 != (0 != dc_pgp_is_valid_key(context, bad_key)) as libc::c_int as libc::c_long {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                941i32,
                b"!dc_pgp_is_valid_key(context, bad_key)\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        j += 1
    }
    dc_key_unref(bad_key);
    let mut public_key: *mut dc_key_t = dc_key_new();
    let mut private_key: *mut dc_key_t = dc_key_new();
    dc_pgp_create_keypair(
        context,
        b"foo@bar.de\x00" as *const u8 as *const libc::c_char,
        public_key,
        private_key,
    );
    if 0 != (0 == dc_pgp_is_valid_key(context, public_key)) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            949i32,
            b"dc_pgp_is_valid_key(context, public_key)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != (0 == dc_pgp_is_valid_key(context, private_key)) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            950i32,
            b"dc_pgp_is_valid_key(context, private_key)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut test_key: *mut dc_key_t = dc_key_new();
    if 0 != (0 == dc_pgp_split_key(context, private_key, test_key)) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            956i32,
            b"dc_pgp_split_key(context, private_key, test_key)\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    dc_key_unref(test_key);
    let mut public_key2: *mut dc_key_t = dc_key_new();
    let mut private_key2: *mut dc_key_t = dc_key_new();
    dc_pgp_create_keypair(
        context,
        b"two@zwo.de\x00" as *const u8 as *const libc::c_char,
        public_key2,
        private_key2,
    );
    if 0 != (0 != dc_key_equals(public_key, public_key2)) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            964i32,
            b"!dc_key_equals(public_key, public_key2)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    let mut original_text: *const libc::c_char =
        b"This is a test\x00" as *const u8 as *const libc::c_char;
    let mut ctext_signed: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut ctext_unsigned: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut ctext_signed_bytes: size_t = 0i32 as size_t;
    let mut ctext_unsigned_bytes: size_t = 0;
    let mut plain_bytes: size_t = 0i32 as size_t;
    let mut keyring: *mut dc_keyring_t = dc_keyring_new();
    dc_keyring_add(keyring, public_key);
    dc_keyring_add(keyring, public_key2);
    let mut ok_0: libc::c_int = dc_pgp_pk_encrypt(
        context,
        original_text as *const libc::c_void,
        strlen(original_text),
        keyring,
        private_key,
        1i32,
        &mut ctext_signed as *mut *mut libc::c_void,
        &mut ctext_signed_bytes,
    );
    if 0 != !(0 != ok_0 && !ctext_signed.is_null() && ctext_signed_bytes > 0) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            975i32,
            b"ok && ctext_signed && ctext_signed_bytes>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(
        ctext_signed as *mut libc::c_char,
        b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        27,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            976i32,
            b"strncmp((char*)ctext_signed, \"-----BEGIN PGP MESSAGE-----\", 27)==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(*(ctext_signed as *mut libc::c_char)
        .offset(ctext_signed_bytes.wrapping_sub(1) as isize) as libc::c_int
        != 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            977i32,
            b"((char*)ctext_signed)[ctext_signed_bytes-1]!=0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    ok_0 = dc_pgp_pk_encrypt(
        context,
        original_text as *const libc::c_void,
        strlen(original_text),
        keyring,
        0 as *const dc_key_t,
        1i32,
        &mut ctext_unsigned as *mut *mut libc::c_void,
        &mut ctext_unsigned_bytes,
    );
    if 0 != !(0 != ok_0 && !ctext_unsigned.is_null() && ctext_unsigned_bytes > 0) as libc::c_int
        as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            981i32,
            b"ok && ctext_unsigned && ctext_unsigned_bytes>0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(
        ctext_unsigned as *mut libc::c_char,
        b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        27,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            982i32,
            b"strncmp((char*)ctext_unsigned, \"-----BEGIN PGP MESSAGE-----\", 27)==0\x00"
                as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(ctext_unsigned_bytes < ctext_signed_bytes) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            983i32,
            b"ctext_unsigned_bytes < ctext_signed_bytes\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    dc_keyring_unref(keyring);
    let mut keyring_0: *mut dc_keyring_t = dc_keyring_new();
    dc_keyring_add(keyring_0, private_key);
    let mut public_keyring: *mut dc_keyring_t = dc_keyring_new();
    dc_keyring_add(public_keyring, public_key);
    let mut public_keyring2: *mut dc_keyring_t = dc_keyring_new();
    dc_keyring_add(public_keyring2, public_key2);
    let mut plain_0: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut valid_signatures: dc_hash_t = dc_hash_t {
        keyClass: 0,
        copyKey: 0,
        count: 0,
        first: 0 as *mut dc_hashelem_t,
        htsize: 0,
        ht: 0 as *mut _ht,
    };
    dc_hash_init(&mut valid_signatures, 3i32, 1i32);
    let mut ok_1: libc::c_int = 0;
    ok_1 = dc_pgp_pk_decrypt(
        context,
        ctext_signed,
        ctext_signed_bytes,
        keyring_0,
        public_keyring,
        1i32,
        &mut plain_0,
        &mut plain_bytes,
        &mut valid_signatures,
    );
    if 0 != !(0 != ok_1 && !plain_0.is_null() && plain_bytes > 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1004i32,
            b"ok && plain && plain_bytes>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(
        plain_0 as *mut libc::c_char,
        original_text,
        strlen(original_text),
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1005i32,
            b"strncmp((char*)plain, original_text, strlen(original_text))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(valid_signatures.count == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1006i32,
            b"dc_hash_cnt(&valid_signatures) == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(plain_0);
    plain_0 = 0 as *mut libc::c_void;
    dc_hash_clear(&mut valid_signatures);
    ok_1 = dc_pgp_pk_decrypt(
        context,
        ctext_signed,
        ctext_signed_bytes,
        keyring_0,
        0 as *const dc_keyring_t,
        1i32,
        &mut plain_0,
        &mut plain_bytes,
        &mut valid_signatures,
    );
    if 0 != !(0 != ok_1 && !plain_0.is_null() && plain_bytes > 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1011i32,
            b"ok && plain && plain_bytes>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(
        plain_0 as *mut libc::c_char,
        original_text,
        strlen(original_text),
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1012i32,
            b"strncmp((char*)plain, original_text, strlen(original_text))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(valid_signatures.count == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1013i32,
            b"dc_hash_cnt(&valid_signatures) == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(plain_0);
    plain_0 = 0 as *mut libc::c_void;
    dc_hash_clear(&mut valid_signatures);
    ok_1 = dc_pgp_pk_decrypt(
        context,
        ctext_signed,
        ctext_signed_bytes,
        keyring_0,
        public_keyring2,
        1i32,
        &mut plain_0,
        &mut plain_bytes,
        &mut valid_signatures,
    );
    if 0 != !(0 != ok_1 && !plain_0.is_null() && plain_bytes > 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1018i32,
            b"ok && plain && plain_bytes>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(
        plain_0 as *mut libc::c_char,
        original_text,
        strlen(original_text),
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1019i32,
            b"strncmp((char*)plain, original_text, strlen(original_text))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(valid_signatures.count == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1020i32,
            b"dc_hash_cnt(&valid_signatures) == 0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(plain_0);
    plain_0 = 0 as *mut libc::c_void;
    dc_hash_clear(&mut valid_signatures);
    dc_keyring_add(public_keyring2, public_key);
    ok_1 = dc_pgp_pk_decrypt(
        context,
        ctext_signed,
        ctext_signed_bytes,
        keyring_0,
        public_keyring2,
        1i32,
        &mut plain_0,
        &mut plain_bytes,
        &mut valid_signatures,
    );
    if 0 != !(0 != ok_1 && !plain_0.is_null() && plain_bytes > 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1026i32,
            b"ok && plain && plain_bytes>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(
        plain_0 as *mut libc::c_char,
        original_text,
        strlen(original_text),
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1027i32,
            b"strncmp((char*)plain, original_text, strlen(original_text))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(valid_signatures.count == 1i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1028i32,
            b"dc_hash_cnt(&valid_signatures) == 1\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    free(plain_0);
    plain_0 = 0 as *mut libc::c_void;
    dc_hash_clear(&mut valid_signatures);
    ok_1 = dc_pgp_pk_decrypt(
        context,
        ctext_unsigned,
        ctext_unsigned_bytes,
        keyring_0,
        public_keyring,
        1i32,
        &mut plain_0,
        &mut plain_bytes,
        &mut valid_signatures,
    );
    if 0 != !(0 != ok_1 && !plain_0.is_null() && plain_bytes > 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1033i32,
            b"ok && plain && plain_bytes>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(
        plain_0 as *mut libc::c_char,
        original_text,
        strlen(original_text),
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1034i32,
            b"strncmp((char*)plain, original_text, strlen(original_text))==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(plain_0);
    plain_0 = 0 as *mut libc::c_void;
    dc_hash_clear(&mut valid_signatures);
    dc_keyring_unref(keyring_0);
    dc_keyring_unref(public_keyring);
    dc_keyring_unref(public_keyring2);
    let mut keyring_1: *mut dc_keyring_t = dc_keyring_new();
    dc_keyring_add(keyring_1, private_key2);
    let mut public_keyring_0: *mut dc_keyring_t = dc_keyring_new();
    dc_keyring_add(public_keyring_0, public_key);
    let mut plain_1: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut ok_2: libc::c_int = dc_pgp_pk_decrypt(
        context,
        ctext_signed,
        ctext_signed_bytes,
        keyring_1,
        public_keyring_0,
        1i32,
        &mut plain_1,
        &mut plain_bytes,
        0 as *mut dc_hash_t,
    );
    if 0 != !(0 != ok_2 && !plain_1.is_null() && plain_bytes > 0) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1053i32,
            b"ok && plain && plain_bytes>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(plain_bytes == strlen(original_text)) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1054i32,
            b"plain_bytes == strlen(original_text)\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strncmp(plain_1 as *const libc::c_char, original_text, plain_bytes) == 0i32)
        as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1055i32,
            b"strncmp(plain, original_text, plain_bytes)==0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(plain_1);
    dc_keyring_unref(keyring_1);
    dc_keyring_unref(public_keyring_0);
    free(ctext_signed);
    free(ctext_unsigned);
    dc_key_unref(public_key2);
    dc_key_unref(private_key2);
    dc_key_unref(public_key);
    dc_key_unref(private_key);
    let mut fingerprint: *mut libc::c_char = dc_normalize_fingerprint(
        b" 1234  567890 \n AbcD abcdef ABCDEF \x00" as *const u8 as *const libc::c_char,
    );
    if 0 != fingerprint.is_null() as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1076i32,
            b"fingerprint\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(strcmp(
        fingerprint,
        b"1234567890ABCDABCDEFABCDEF\x00" as *const u8 as *const libc::c_char,
    ) == 0i32) as libc::c_int as libc::c_long
    {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                .as_ptr(),
            b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
            1077i32,
            b"strcmp(fingerprint, \"1234567890ABCDABCDEFABCDEF\") == 0\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    free(fingerprint as *mut libc::c_void);
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
            ) == 0i32) as libc::c_int as libc::c_long
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
        if 0 != res.is_null() as libc::c_int as libc::c_long {
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
            as libc::c_int as libc::c_long
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
        if 0 != res.is_null() as libc::c_int as libc::c_long {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                1094i32,
                b"res\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !((*res).state == 320i32) as libc::c_int as libc::c_long {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 17], &[libc::c_char; 17]>(b"stress_functions\x00"))
                    .as_ptr(),
                b"../cmdline/stress.c\x00" as *const u8 as *const libc::c_char,
                1095i32,
                b"res->state == DC_QR_ADDR\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        if 0 != !((*res).id != 0i32 as libc::c_uint) as libc::c_int as libc::c_long {
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

unsafe extern "C" fn cb(
    _context: &dc_context_t,
    _event: Event,
    _data1: uintptr_t,
    _data2: uintptr_t,
) -> uintptr_t {
    0
}

#[test]
fn run_stress_tests() {
    unsafe {
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

        stress_functions(&ctx)
    }
}
