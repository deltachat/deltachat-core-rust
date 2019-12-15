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
