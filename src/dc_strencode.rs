use itertools::Itertools;
use std::borrow::Cow;
use std::ffi::CString;
use std::ptr;

use charset::Charset;
use libc::free;
use mmime::mailmime::decode::mailmime_encoded_phrase_parse;
use mmime::other::*;
use percent_encoding::{percent_decode, utf8_percent_encode, AsciiSet, CONTROLS};

use crate::dc_tools::*;

/**
 * Encode non-ascii-strings as `=?UTF-8?Q?Bj=c3=b6rn_Petersen?=`.
 * Belongs to RFC 2047: https://tools.ietf.org/html/rfc2047
 *
 * We do not fold at position 72; this would result in empty words as `=?utf-8?Q??=` which are correct,
 * but cannot be displayed by some mail programs (eg. Android Stock Mail).
 * however, this is not needed, as long as _one_ word is not longer than 72 characters.
 * _if_ it is, the display may get weird.  This affects the subject only.
 * the best solution wor all this would be if libetpan encodes the line as only libetpan knowns when a header line is full.
 *
 * @param to_encode Null-terminated UTF-8-string to encode.
 * @return Returns the encoded string which must be free()'d when no longed needed.
 *     On errors, NULL is returned.
 */
pub fn dc_encode_header_words(input: impl AsRef<str>) -> String {
    let mut result = String::default();
    for (_, group) in &input.as_ref().chars().group_by(|c| c.is_whitespace()) {
        let word: String = group.collect();
        result.push_str(&quote_word(&word.as_bytes()));
    }

    result
}

fn must_encode(byte: u8) -> bool {
    static SPECIALS: &[u8] = b",:!\"#$@[\\]^`{|}~=?_";

    SPECIALS.into_iter().any(|b| *b == byte)
}

fn quote_word(word: &[u8]) -> String {
    let mut result = String::default();
    let mut encoded = false;

    for byte in word {
        let byte = *byte;
        if byte >= 128 || must_encode(byte) {
            result.push_str(&format!("={:2X}", byte));
            encoded = true;
        } else if byte == b' ' {
            result.push('_');
            encoded = true;
        } else {
            result.push(byte as _);
        }
    }

    if encoded {
        result = format!("=?utf-8?Q?{}?=", &result);
    }
    result
}

/* ******************************************************************************
 * Encode/decode header words, RFC 2047
 ******************************************************************************/

pub(crate) fn dc_decode_header_words(input: &str) -> String {
    static FROM_ENCODING: &[u8] = b"iso-8859-1\x00";
    static TO_ENCODING: &[u8] = b"utf-8\x00";
    let mut out = ptr::null_mut();
    let mut cur_token = 0;
    let input_c = CString::yolo(input);
    unsafe {
        let r = mailmime_encoded_phrase_parse(
            FROM_ENCODING.as_ptr().cast(),
            input_c.as_ptr(),
            input.len(),
            &mut cur_token,
            TO_ENCODING.as_ptr().cast(),
            &mut out,
        );
        if r as u32 != MAILIMF_NO_ERROR || out.is_null() {
            input.to_string()
        } else {
            let res = to_string_lossy(out);
            free(out.cast());
            res
        }
    }
}

pub fn dc_needs_ext_header(to_check: impl AsRef<str>) -> bool {
    let to_check = to_check.as_ref();

    if to_check.is_empty() {
        return false;
    }

    to_check.chars().any(|c| {
        !(c.is_ascii_alphanumeric()
            || c == '-'
            || c == '_'
            || c == '_'
            || c == '.'
            || c == '~'
            || c == '%')
    })
}

const EXT_ASCII_ST: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'-')
    .add(b'_')
    .add(b'.')
    .add(b'~')
    .add(b'%');

/// Encode an UTF-8 string to the extended header format.
pub fn dc_encode_ext_header(to_encode: impl AsRef<str>) -> String {
    let encoded = utf8_percent_encode(to_encode.as_ref(), &EXT_ASCII_ST);
    format!("utf-8''{}", encoded)
}

/// Decode an extended-header-format strings to UTF-8.
pub fn dc_decode_ext_header(to_decode: &[u8]) -> Cow<str> {
    if let Some(index) = bytes!(b'\'').find(to_decode) {
        let (charset, rest) = to_decode.split_at(index);
        if !charset.is_empty() {
            // skip language
            if let Some(index2) = bytes!(b'\'').find(&rest[1..]) {
                let decoded = percent_decode(&rest[index2 + 2..]);

                if charset != b"utf-8" && charset != b"UTF-8" {
                    if let Some(encoding) = Charset::for_label(charset) {
                        let bytes = decoded.collect::<Vec<u8>>();
                        let (res, _, _) = encoding.decode(&bytes);
                        return Cow::Owned(res.into_owned());
                    } else {
                        return decoded.decode_utf8_lossy();
                    }
                } else {
                    return decoded.decode_utf8_lossy();
                }
            }
        }
    }

    String::from_utf8_lossy(to_decode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_decode_header_words() {
        assert_eq!(
            dc_decode_header_words("=?utf-8?B?dGVzdMOkw7bDvC50eHQ=?="),
            std::string::String::from_utf8(b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt".to_vec()).unwrap(),
        );

        assert_eq!(dc_decode_header_words("just ascii test"), "just ascii test");

        assert_eq!(dc_encode_header_words("abcdef"), "abcdef");

        let r = dc_encode_header_words(
            std::string::String::from_utf8(b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt".to_vec()).unwrap(),
        );
        assert!(r.starts_with("=?utf-8"));

        assert_eq!(
            dc_decode_header_words(&r),
            std::string::String::from_utf8(b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt".to_vec()).unwrap(),
        );

        assert_eq!(
                dc_decode_header_words("=?ISO-8859-1?Q?attachment=3B=0D=0A_filename=3D?= =?ISO-8859-1?Q?=22test=E4=F6=FC=2Etxt=22=3B=0D=0A_size=3D39?="),
                std::string::String::from_utf8(b"attachment;\r\n filename=\"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\";\r\n size=39".to_vec()).unwrap(),
            );
    }

    #[test]
    fn test_dc_encode_ext_header() {
        let buf1 = dc_encode_ext_header("Björn Petersen");
        assert_eq!(&buf1, "utf-8\'\'Bj%C3%B6rn%20Petersen");
        let buf2 = dc_decode_ext_header(buf1.as_bytes());
        assert_eq!(&buf2, "Björn Petersen",);

        let buf1 = dc_decode_ext_header(b"iso-8859-1\'en\'%A3%20rates");
        assert_eq!(buf1, "£ rates",);

        let buf1 = dc_decode_ext_header(b"wrong\'format");
        assert_eq!(buf1, "wrong\'format",);

        let buf1 = dc_decode_ext_header(b"\'\'");
        assert_eq!(buf1, "\'\'");

        let buf1 = dc_decode_ext_header(b"x\'\'");
        assert_eq!(buf1, "");

        let buf1 = dc_decode_ext_header(b"\'");
        assert_eq!(buf1, "\'");

        let buf1 = dc_decode_ext_header(b"");
        assert_eq!(buf1, "");

        // regressions
        assert_eq!(
            dc_decode_ext_header(dc_encode_ext_header("%0A").as_bytes()),
            "%0A"
        );
    }

    #[test]
    fn test_dc_needs_ext_header() {
        assert_eq!(dc_needs_ext_header("Björn"), true);
        assert_eq!(dc_needs_ext_header("Bjoern"), false);
        assert_eq!(dc_needs_ext_header(""), false);
        assert_eq!(dc_needs_ext_header(" "), true);
        assert_eq!(dc_needs_ext_header("a b"), true);
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_ext_header_roundtrip(buf: String) {
            let encoded = dc_encode_ext_header(&buf);
            let decoded = dc_decode_ext_header(encoded.as_bytes());
            assert_eq!(buf, decoded);
        }

        #[test]
        fn test_ext_header_decode_anything(buf: Vec<u8>) {
            // make sure this never panics
            let _decoded = dc_decode_ext_header(&buf);
        }

        #[test]
        fn test_dc_header_roundtrip(input: String) {
            let encoded = dc_encode_header_words(&input);
            let decoded = dc_decode_header_words(&encoded);

            assert_eq!(input, decoded);
        }
    }
}
