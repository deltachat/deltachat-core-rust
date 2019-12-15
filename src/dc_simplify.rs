use crate::dehtml::*;

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
            || line == "-----"
            || line == "_____"
            || line == "====="
            || line == "*****"
            || line == "~~~~~"
        {
            return (&lines[..ix], true);
        }
    }
    (lines, false)
}

fn split_lines(buf: &str) -> Vec<&str> {
    buf.split('\n').collect()
}

/// Simplify and normalise text: Remove quotes, signatures, unnecessary
/// lineends etc.
pub fn simplify(input: &str, is_html: bool, is_chat_message: bool) -> (String, bool) {
    let mut out = if is_html {
        dehtml(input)
    } else {
        input.to_string()
    };

    out.retain(|c| c != '\r');
    let lines = split_lines(&out);
    let (lines, is_forwarded) = skip_forward_header(&lines);
    (simplify_plain_text(lines, is_chat_message), is_forwarded)
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

/**
 * Simplify Plain Text
 */
#[allow(non_snake_case, clippy::mut_range_bound, clippy::needless_range_loop)]
fn simplify_plain_text(lines: &[&str], is_chat_message: bool) -> String {
    /* This function ...
    ... removes all text after the line `-- ` (footer mark)
    ... removes full quotes at the beginning and at the end of the text -
        these are all lines starting with the character `>`
    ... remove a non-empty line before the removed quote (contains sth. like "On 2.9.2016, Bjoern wrote:" in different formats and lanugages) */
    /* split the given buffer into lines */
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

    let is_cut_at_end = has_nonstandard_footer || has_bottom_quote;
    let is_cut_at_begin = has_top_quote;

    /* re-create buffer from the remaining lines */
    let mut ret = String::new();
    if is_cut_at_begin {
        ret += "[...]";
    }
    /* we write empty lines only in case and non-empty line follows */
    let mut pending_linebreaks = 0;
    let mut empty_body = true;
    for l in 0..lines.len() {
        let line = lines[l];
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
            let output = simplify_plain_text(&split_lines(&input), true);
            assert!(output.split('\n').all(|s| s != "-- "));
        }
    }

    #[test]
    fn test_simplify_trim() {
        let html = "\r\r\nline1<br>\r\n\r\n\r\rline2\n\r";
        let (plain, is_forwarded) = simplify(html, true, false);

        assert_eq!(plain, "line1\nline2");
        assert!(!is_forwarded);
    }

    #[test]
    fn test_simplify_parse_href() {
        let html = "<a href=url>text</a";
        let (plain, is_forwarded) = simplify(html, true, false);

        assert_eq!(plain, "[text](url)");
        assert!(!is_forwarded);
    }

    #[test]
    fn test_simplify_bold_text() {
        let html = "<!DOCTYPE name [<!DOCTYPE ...>]><!-- comment -->text <b><?php echo ... ?>bold</b><![CDATA[<>]]>";
        let (plain, is_forwarded) = simplify(html, true, false);

        assert_eq!(plain, "text *bold*<>");
        assert!(!is_forwarded);
    }

    #[test]
    fn test_simplify_html_encoded() {
        let html =
                "&lt;&gt;&quot;&apos;&amp; &auml;&Auml;&ouml;&Ouml;&uuml;&Uuml;&szlig; foo&AElig;&ccedil;&Ccedil; &diams;&lrm;&rlm;&zwnj;&noent;&zwj;";

        let (plain, is_forwarded) = simplify(html, true, false);

        assert_eq!(
            plain,
            "<>\"\'& äÄöÖüÜß fooÆçÇ \u{2666}\u{200e}\u{200f}\u{200c}&noent;\u{200d}"
        );
        assert!(!is_forwarded);
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
}
