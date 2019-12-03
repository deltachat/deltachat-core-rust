use mailparse::ParsedMail;

use crate::error::Error;

pub fn parse_message_id(message_id: &[u8]) -> Result<String, Error> {
    let value = std::str::from_utf8(message_id)?;
    let addrs = mailparse::addrparse(value)
        .map_err(|err| format_err!("failed to parse message id {:?}", err))?;

    if let Some(info) = addrs.extract_single_info() {
        return Ok(info.addr);
    }

    bail!("could not parse message_id: {}", value);
}

/// Returns a reference to the encrypted payload and validates the autocrypt structure.
pub fn get_autocrypt_mime<'a, 'b>(mail: &'a ParsedMail<'b>) -> Result<&'a ParsedMail<'b>, Error> {
    ensure!(
        mail.ctype.mimetype == "multipart/encrypted",
        "Not a multipart/encrypted message: {}",
        mail.ctype.mimetype
    );
    ensure!(
        mail.subparts.len() == 2,
        "Invalid Autocrypt Level 1 Mime Parts"
    );

    ensure!(
        mail.subparts[0].ctype.mimetype == "application/pgp-encrypted",
        "Invalid Autocrypt Level 1 version part: {:?}",
        mail.subparts[0].ctype,
    );

    ensure!(
        mail.subparts[1].ctype.mimetype == "application/octet-stream",
        "Invalid Autocrypt Level 1 encrypted part: {:?}",
        mail.subparts[1].ctype
    );

    Ok(&mail.subparts[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_id() {
        assert_eq!(
            parse_message_id(b"Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org").unwrap(),
            "Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org"
        );
        assert_eq!(
            parse_message_id(b"<Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org>").unwrap(),
            "Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org"
        );
    }
}
