// protect lines starting with `--` against being treated as a footer.
// for that, we insert a ZERO WIDTH SPACE (ZWSP, 0x200B);
// this should be invisible on most systems and there is no need to unescape it again
// (which won't be done by non-deltas anyway)
//
// this escapes a bit more than actually needed by delta (eg. also lines as "-- footer"),
// but for non-delta-compatibility, that seems to be better.
// (to be only compatible with delta, only "[\r\n|\n]-- {0,2}[\r\n|\n]" needs to be replaced)
pub fn escape_message_footer_marks(text: &str) -> String {
    if text.starts_with("--") {
        "-\u{200B}-".to_string() + &text[2..].replace("\n--", "\n-\u{200B}-")
    } else {
        text.replace("\n--", "\n-\u{200B}-")
    }
}

/// Remove standard (RFC 3676, §4.3) footer if it is found.
fn remove_message_footer<'a>(lines: &'a [&str]) -> &'a [&'a str] {
    let mut nearly_standard_footer = None;
    for (ix, &line) in lines.iter().enumerate() {
        match line {
            // some providers encode `-- ` to `-- =20` which results in `--  `
            "-- " | "--  " => return &lines[..ix],
            // some providers encode `-- ` to `=2D-` which results in only `--`;
            // use that only when no other footer is found
            // and if the line before is empty and the line after is not empty
            "--" => {
                if (ix == 0 || lines[ix - 1] == "") && ix != lines.len() - 1 && lines[ix + 1] != ""
                {
                    nearly_standard_footer = Some(ix);
                }
            }
            _ => (),
        }
    }
    if let Some(ix) = nearly_standard_footer {
        return &lines[..ix];
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

    let original_lines = &lines;

    let lines = remove_message_footer(lines);

    let text = if is_chat_message {
        render_message(lines, false, false)
    } else {
        let (lines, has_nonstandard_footer) = remove_nonstandard_footer(lines);
        let (lines, has_bottom_quote) = remove_bottom_quote(lines);
        let (lines, has_top_quote) = remove_top_quote(lines);

        if lines.iter().all(|it| it.trim().is_empty()) {
            render_message(original_lines, false, false)
        } else {
            render_message(
                lines,
                has_top_quote,
                has_nonstandard_footer || has_bottom_quote,
            )
        }
    };
    (text, is_forwarded)
}

/// Skips "forwarded message" header.
/// Returns message body lines and a boolean indicating whether
/// a message is forwarded or not.
fn skip_forward_header<'a>(lines: &'a [&str]) -> (&'a [&'a str], bool) {
    match lines {
        ["---------- Forwarded message ----------", first_line, "", rest @ ..]
            if first_line.starts_with("From: ") =>
        {
            (rest, true)
        }
        _ => (lines, false),
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
    // redo escaping done by escape_message_footer_marks()
    ret.replace("\u{200B}", "")
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
    fn test_dont_remove_whole_message() {
        let input = "\n------\nFailed\n------\n\nUh-oh, this workflow did not succeed!\n\nlots of other text".to_string();
        let (plain, is_forwarded) = simplify(input, false);
        assert_eq!(
            plain,
            "------\nFailed\n------\n\nUh-oh, this workflow did not succeed!\n\nlots of other text"
        );
        assert!(!is_forwarded);
    }

    #[test]
    fn test_chat_message() {
        let input = "Hi! How are you?\n\n---\n\nI am good.\n-- \nSent with my Delta Chat Messenger: https://delta.chat".to_string();
        let (plain, is_forwarded) = simplify(input, true);
        assert_eq!(plain, "Hi! How are you?\n\n---\n\nI am good.");
        assert!(!is_forwarded);
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

    #[test]
    fn test_escape_message_footer_marks() {
        let esc = escape_message_footer_marks("--\n--text --in line");
        assert_eq!(esc, "-\u{200B}-\n-\u{200B}-text --in line");

        let esc = escape_message_footer_marks("--\r\n--text");
        assert_eq!(esc, "-\u{200B}-\r\n-\u{200B}-text");
    }

    #[test]
    fn test_remove_message_footer() {
        let input = "text\n--\nno footer".to_string();
        let (plain, _) = simplify(input, true);
        assert_eq!(plain, "text\n--\nno footer");

        let input = "text\n\n--\n\nno footer".to_string();
        let (plain, _) = simplify(input, true);
        assert_eq!(plain, "text\n\n--\n\nno footer");

        let input = "text\n\n-- no footer\n\n".to_string();
        let (plain, _) = simplify(input, true);
        assert_eq!(plain, "text\n\n-- no footer");

        let input = "text\n\n--\nno footer\n-- \nfooter".to_string();
        let (plain, _) = simplify(input, true);
        assert_eq!(plain, "text\n\n--\nno footer");

        let input = "text\n\n--\ntreated as footer when unescaped".to_string();
        let (plain, _) = simplify(input.clone(), true);
        assert_eq!(plain, "text"); // see remove_message_footer() for some explanations
        let escaped = escape_message_footer_marks(&input);
        let (plain, _) = simplify(escaped, true);
        assert_eq!(plain, "text\n\n--\ntreated as footer when unescaped");

        let input = "--\ntreated as footer when unescaped".to_string();
        let (plain, _) = simplify(input.clone(), true);
        assert_eq!(plain, ""); // see remove_message_footer() for some explanations
        let escaped = escape_message_footer_marks(&input);
        let (plain, _) = simplify(escaped, true);
        assert_eq!(plain, "--\ntreated as footer when unescaped");
    }
}
