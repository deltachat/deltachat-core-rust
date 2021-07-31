///! # format=flowed support
///!
///! Format=flowed is defined in
///! [RFC 3676](https://tools.ietf.org/html/rfc3676).
///!
///! Older [RFC 2646](https://tools.ietf.org/html/rfc2646) is used
///! during formatting, i.e., DelSp parameter introduced in RFC 3676
///! is assumed to be set to "no".
///!
///! For received messages, DelSp parameter is honoured.

/// Wraps line to 72 characters using format=flowed soft breaks.
///
/// 72 characters is the limit recommended by RFC 3676.
///
/// The function breaks line only after SP and before non-whitespace
/// characters. It also does not insert breaks before ">" to avoid the
/// need to do space stuffing (see RFC 3676) for quotes.
///
/// If there are long words, line may still exceed the limits on line
/// length. However, this should be rare and should not result in
/// immediate mail rejection: SMTP (RFC 2821) limit is 998 characters,
/// and Spam Assassin limit is 78 characters.
fn format_line_flowed(line: &str, prefix: &str) -> String {
    let mut result = String::new();
    let mut buffer = prefix.to_string();
    let mut after_space = false;

    for c in line.chars() {
        if c == ' ' {
            buffer.push(c);
            after_space = true;
        } else if c == '>' {
            if buffer.is_empty() {
                // Space stuffing, see RFC 3676
                buffer.push(' ');
            }
            buffer.push(c);
            after_space = false;
        } else {
            if after_space && buffer.len() >= 72 && !c.is_whitespace() {
                // Flush the buffer and insert soft break (SP CRLF).
                result += &buffer;
                result += "\r\n";
                buffer = prefix.to_string();
            }
            buffer.push(c);
            after_space = false;
        }
    }
    result + &buffer
}

fn format_flowed_prefix(text: &str, prefix: &str) -> String {
    let mut result = String::new();

    for line in text.split('\n') {
        if !result.is_empty() {
            result += "\r\n";
        }
        let line = line.trim_end();
        if prefix.len() + line.len() > 78 {
            result += &format_line_flowed(line, prefix);
        } else {
            result += prefix;
            if prefix.is_empty() && line.starts_with('>') {
                // Space stuffing, see RFC 3676
                result.push(' ');
            }
            result += line;
        }
    }
    result
}

/// Returns text formatted according to RFC 3767 (format=flowed).
///
/// This function accepts text separated by LF, but returns text
/// separated by CRLF.
///
/// RFC 2646 technique is used to insert soft line breaks, so DelSp
/// SHOULD be set to "no" when sending.
pub fn format_flowed(text: &str) -> String {
    format_flowed_prefix(text, "")
}

/// Same as format_flowed(), but adds "> " prefix to each line.
pub fn format_flowed_quote(text: &str) -> String {
    format_flowed_prefix(text, "> ")
}

/// Joins lines in format=flowed text.
///
/// Lines must be separated by single LF.
///
/// Quote processing is not supported, it is assumed that they are
/// deleted during simplification.
///
/// Signature separator line is not processed here, it is assumed to
/// be stripped beforehand.
pub fn unformat_flowed(text: &str, delsp: bool) -> String {
    let mut result = String::new();
    let mut skip_newline = true;

    for line in text.split('\n') {
        // Revert space-stuffing
        let line = line.strip_prefix(' ').unwrap_or(line);

        if !skip_newline {
            result.push('\n');
        }

        if let Some(line) = line.strip_suffix(' ') {
            // Flowed line
            result += line;
            if !delsp {
                result.push(' ');
            }
            skip_newline = true;
        } else {
            // Fixed line
            result += line;
            skip_newline = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_flowed() {
        let text = "Foo bar baz";
        assert_eq!(format_flowed(text), "Foo bar baz");

        let text = "This is the Autocrypt Setup Message used to transfer your key between clients.\n\
                    \n\
                    To decrypt and use your key, open the message in an Autocrypt-compliant client and enter the setup code presented on the generating device.";
        let expected = "This is the Autocrypt Setup Message used to transfer your key between clients.\r\n\
                        \r\n\
                        To decrypt and use your key, open the message in an Autocrypt-compliant \r\n\
                        client and enter the setup code presented on the generating device.";
        assert_eq!(format_flowed(text), expected);

        let text = "> Not a quote";
        assert_eq!(format_flowed(text), " > Not a quote");

        // Test space stuffing of wrapped lines
        let text = "> This is the Autocrypt Setup Message used to transfer your key between clients.\n\
                    >                               \n\
                    > To decrypt and use your key, open the message in an Autocrypt-compliant client and enter the setup code presented on the generating device.";
        let expected = "\x20> This is the Autocrypt Setup Message used to transfer your key between \r\n\
                        clients.\r\n\
                        \x20>\r\n\
                        \x20> To decrypt and use your key, open the message in an Autocrypt-compliant \r\n\
                        client and enter the setup code presented on the generating device.";
        assert_eq!(format_flowed(text), expected);
    }

    #[test]
    fn test_unformat_flowed() {
        let text = "this is a very long message that should be wrapped using format=flowed and \n\
            unwrapped on the receiver";
        let expected =
            "this is a very long message that should be wrapped using format=flowed and \
                        unwrapped on the receiver";
        assert_eq!(unformat_flowed(text, false), expected);
    }

    #[test]
    fn test_format_flowed_quote() {
        let quote = "this is a quoted line";
        let expected = "> this is a quoted line";
        assert_eq!(format_flowed_quote(quote), expected);

        let quote = "> foo bar baz";
        let expected = "> > foo bar baz";
        assert_eq!(format_flowed_quote(quote), expected);

        let quote = "this is a very long quote that should be wrapped using format=flowed and unwrapped on the receiver";
        let expected =
            "> this is a very long quote that should be wrapped using format=flowed and \r\n\
            > unwrapped on the receiver";
        assert_eq!(format_flowed_quote(quote), expected);
    }
}
