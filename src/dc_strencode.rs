use itertools::Itertools;

/// Encode non-ascii-strings as `=?UTF-8?Q?Bj=c3=b6rn_Petersen?=`.
/// Belongs to RFC 2047: https://tools.ietf.org/html/rfc2047
///
/// We do not fold at position 72; this would result in empty words as `=?utf-8?Q??=` which are correct,
/// but cannot be displayed by some mail programs (eg. Android Stock Mail).
/// however, this is not needed, as long as _one_ word is not longer than 72 characters.
/// _if_ it is, the display may get weird.  This affects the subject only.
/// the best solution wor all this would be if libetpan encodes the line as only libetpan knowns when a header line is full.
///
/// @param to_encode Null-terminated UTF-8-string to encode.
/// @return Returns the encoded string which must be free()'d when no longed needed.
///     On errors, NULL is returned.
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

    SPECIALS.iter().any(|b| *b == byte)
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

pub fn dc_needs_ext_header(to_check: impl AsRef<str>) -> bool {
    let to_check = to_check.as_ref();

    if to_check.is_empty() {
        return false;
    }

    to_check.chars().any(|c| {
        !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' && c != '~' && c != '%'
    })
}
