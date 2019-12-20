/// Remove standard (RFC 3676, §4.3) footer if it is found.
fn remove_message_footer<'a>(lines: &'a [&str]) -> &'a [&'a str] {
    for (ix, &line) in lines.iter().enumerate() {
        // quoted-printable may encode `-- ` to `-- =20` which is converted
        // back to `--  `
        match line {
            "-- " | "--  " => return &lines[..ix],
            _ => (),
        }
    }
    lines
}

/// Remove nonstandard footer and a boolean indicating whether such
/// footer was removed.
fn remove_nonstandard_footer<'a>(lines: &'a [&str]) -> (&'a [&'a str], bool) {
    for (ix, &line) in lines.iter().enumerate() {
        if line == "--"
            || line == "---"
            || line == "----"
            || line.starts_with("-----")
            || line.starts_with("_____")
            || line.starts_with("=====")
            || line.starts_with("*****")
            || line.starts_with("~~~~~")
        {
            return (&lines[..ix], true);
        }
    }
    (lines, false)
}

fn split_lines(buf: &str) -> Vec<&str> {
    buf.split('\n').collect()
}

/// Simplify message text for chat display.
/// Remove quotes, signatures, trailing empty lines etc.
pub fn simplify(mut input: String, is_chat_message: bool) -> (String, bool) {
    input.retain(|c| c != '\r');
    let lines = split_lines(&input);
    let (lines, is_forwarded) = skip_forward_header(&lines);

    let lines = remove_message_footer(lines);
    let (lines, has_nonstandard_footer) = remove_nonstandard_footer(lines);
    let (lines, has_bottom_quote) = if !is_chat_message {
        remove_bottom_quote(lines)
    } else {
        (lines, false)
    };
    let (lines, has_top_quote) = if !is_chat_message {
        remove_top_quote(lines)
    } else {
        (lines, false)
    };

    // re-create buffer from the remaining lines
    let text = render_message(
        lines,
        has_top_quote,
        has_nonstandard_footer || has_bottom_quote,
    );
    (text, is_forwarded)
}

/// Skips "forwarded message" header.
/// Returns message body lines and a boolean indicating whether
/// a message is forwarded or not.
fn skip_forward_header<'a>(lines: &'a [&str]) -> (&'a [&'a str], bool) {
    if lines.len() >= 3
        && lines[0] == "---------- Forwarded message ----------"
        && lines[1].starts_with("From: ")
        && lines[2].is_empty()
    {
        (&lines[3..], true)
    } else {
        (lines, false)
    }
}

fn remove_bottom_quote<'a>(lines: &'a [&str]) -> (&'a [&'a str], bool) {
    let mut last_quoted_line = None;
    for (l, line) in lines.iter().enumerate().rev() {
        if is_plain_quote(line) {
            last_quoted_line = Some(l)
        } else if !is_empty_line(line) {
            break;
        }
    }
    if let Some(mut l_last) = last_quoted_line {
        if l_last > 1 && is_empty_line(lines[l_last - 1]) {
            l_last -= 1
        }
        if l_last > 1 {
            let line = lines[l_last - 1];
            if is_quoted_headline(line) {
                l_last -= 1
            }
        }
        (&lines[..l_last], true)
    } else {
        (lines, false)
    }
}

fn remove_top_quote<'a>(lines: &'a [&str]) -> (&'a [&'a str], bool) {
    let mut last_quoted_line = None;
    let mut has_quoted_headline = false;
    for (l, line) in lines.iter().enumerate() {
        if is_plain_quote(line) {
            last_quoted_line = Some(l)
        } else if !is_empty_line(line) {
            if is_quoted_headline(line) && !has_quoted_headline && last_quoted_line.is_none() {
                has_quoted_headline = true
            } else {
                /* non-quoting line found */
                break;
            }
        }
    }
    if let Some(last_quoted_line) = last_quoted_line {
        (&lines[last_quoted_line + 1..], true)
    } else {
        (lines, false)
    }
}

fn render_message(lines: &[&str], is_cut_at_begin: bool, is_cut_at_end: bool) -> String {
    let mut ret = String::new();
    if is_cut_at_begin {
        ret += "[...]";
    }
    /* we write empty lines only in case and non-empty line follows */
    let mut pending_linebreaks = 0;
    let mut empty_body = true;
    for line in lines {
        if is_empty_line(line) {
            pending_linebreaks += 1
        } else {
            if !empty_body {
                if pending_linebreaks > 2 {
                    pending_linebreaks = 2
                }
                while 0 != pending_linebreaks {
                    ret += "\n";
                    pending_linebreaks -= 1
                }
            }
            // the incoming message might contain invalid UTF8
            ret += line;
            empty_body = false;
            pending_linebreaks = 1
        }
    }
    if is_cut_at_end && (!is_cut_at_begin || !empty_body) {
        ret += " [...]";
    }
    ret
}

/**
 * Tools
 */
fn is_empty_line(buf: &str) -> bool {
    // XXX: can it be simplified to buf.chars().all(|c| c.is_whitespace())?
    //
    // Strictly speaking, it is not equivalent (^A is not whitespace, but less than ' '),
    // but having control sequences in email body?!
    //
    // See discussion at: https://github.com/deltachat/deltachat-core-rust/pull/402#discussion_r317062392
    for c in buf.chars() {
        if c > ' ' {
            return false;
        }
    }

    true
}

fn is_quoted_headline(buf: &str) -> bool {
    /* This function may be called for the line _directly_ before a quote.
    The function checks if the line contains sth. like "On 01.02.2016, xy@z wrote:" in various languages.
    - Currently, we simply check if the last character is a ':'.
    - Checking for the existence of an email address may fail (headlines may show the user's name instead of the address) */

    buf.len() <= 80 && buf.ends_with(':')
}

fn is_plain_quote(buf: &str) -> bool {
    buf.starts_with('>')
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        // proptest does not support [[:graphical:][:space:]] regex.
        fn test_simplify_plain_text_fuzzy(input in "[!-~\t \n]+") {
            let (output, _is_forwarded) = simplify(input, true);
            assert!(output.split('\n').all(|s| s != "-- "));
        }
    }

    #[test]
    fn test_simplify_trim() {
        let input = "line1\n\r\r\rline2".to_string();
        let (plain, is_forwarded) = simplify(input, false);

        assert_eq!(plain, "line1\nline2");
        assert!(!is_forwarded);
    }

    #[test]
    fn test_simplify_forwarded_message() {
        let input = "---------- Forwarded message ----------\r\nFrom: test@example.com\r\n\r\nForwarded message\r\n-- \r\nSignature goes here".to_string();
        let (plain, is_forwarded) = simplify(input, false);

        assert_eq!(plain, "Forwarded message");
        assert!(is_forwarded);
    }

    #[test]
    fn test_simplify_utilities() {
        assert!(is_empty_line(" \t"));
        assert!(is_empty_line(""));
        assert!(is_empty_line(" \r"));
        assert!(!is_empty_line(" x"));
        assert!(is_plain_quote("> hello world"));
        assert!(is_plain_quote(">>"));
        assert!(!is_plain_quote("Life is pain"));
        assert!(!is_plain_quote(""));
    }

    #[test]
    fn test_remove_top_quote() {
        let (lines, has_top_quote) = remove_top_quote(&["> first", "> second"]);
        assert!(lines.is_empty());
        assert!(has_top_quote);

        let (lines, has_top_quote) = remove_top_quote(&["> first", "> second", "not a quote"]);
        assert_eq!(lines, &["not a quote"]);
        assert!(has_top_quote);

        let (lines, has_top_quote) = remove_top_quote(&["not a quote", "> first", "> second"]);
        assert_eq!(lines, &["not a quote", "> first", "> second"]);
        assert!(!has_top_quote);
    }
}
