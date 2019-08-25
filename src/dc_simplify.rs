use crate::dc_dehtml::*;

#[derive(Copy, Clone)]
pub struct Simplify {
    pub is_forwarded: bool,
}

/// Return index of footer line in vector of message lines, or vector length if
/// no footer is found.
///
/// Also return whether not-standard (rfc3676, §4.3) footer is found.
fn find_message_footer(lines: &[&str]) -> (usize, bool) {
    for ix in 0..lines.len() {
        let line = lines[ix];

        // quoted-printable may encode `-- ` to `-- =20` which is converted
        // back to `--  `
        match line.as_ref() {
            "-- " | "--  " => return (ix, false),
            "--" | "---" | "----" => return (ix, true),
            _ => (),
        }
    }
    return (lines.len(), false);
}

impl Simplify {
    pub fn new() -> Self {
        Simplify {
            is_forwarded: false,
        }
    }

    /// Simplify and normalise text: Remove quotes, signatures, unnecessary
    /// lineends etc.
    /// The data returned from simplify() must be free()'d when no longer used.
    pub fn simplify(&mut self, input: &str, is_html: bool, is_msgrmsg: bool) -> String {
        let mut out = if is_html {
            dc_dehtml(input)
        } else {
            input.to_string()
        };

        out.retain(|c| c != '\r');
        out = self.simplify_plain_text(&out, is_msgrmsg);
        out.retain(|c| c != '\r');

        out
    }

    /**
     * Simplify Plain Text
     */
    #[allow(non_snake_case)]
    fn simplify_plain_text(&mut self, buf_terminated: &str, is_msgrmsg: bool) -> String {
        /* This function ...
        ... removes all text after the line `-- ` (footer mark)
        ... removes full quotes at the beginning and at the end of the text -
            these are all lines starting with the character `>`
        ... remove a non-empty line before the removed quote (contains sth. like "On 2.9.2016, Bjoern wrote:" in different formats and lanugages) */
        /* split the given buffer into lines */
        let lines: Vec<_> = buf_terminated.split('\n').collect();
        let mut l_first: usize = 0;
        let mut is_cut_at_begin = false;
        let (mut l_last, mut is_cut_at_end) = find_message_footer(&lines);

        if l_last > l_first + 2 {
            let line0 = lines[l_first];
            let line1 = lines[l_first + 1];
            let line2 = lines[l_first + 2];
            if line0 == "---------- Forwarded message ----------"
                && line1.starts_with("From: ")
                && line2.is_empty()
            {
                self.is_forwarded = true;
                l_first += 3
            }
        }
        for l in l_first..l_last {
            let line = lines[l];
            if line == "-----"
                || line == "_____"
                || line == "====="
                || line == "*****"
                || line == "~~~~~"
            {
                l_last = l;
                is_cut_at_end = true;
                /* done */
                break;
            }
        }
        if !is_msgrmsg {
            let mut l_lastQuotedLine = None;
            for l in (l_first..l_last).rev() {
                let line = lines[l];
                if is_plain_quote(line) {
                    l_lastQuotedLine = Some(l)
                } else if !is_empty_line(line) {
                    break;
                }
            }
            if l_lastQuotedLine.is_some() {
                l_last = l_lastQuotedLine.unwrap();
                is_cut_at_end = true;
                if l_last > 1 {
                    if is_empty_line(lines[l_last - 1]) {
                        l_last -= 1
                    }
                }
                if l_last > 1 {
                    let line = lines[l_last - 1];
                    if is_quoted_headline(line) {
                        l_last -= 1
                    }
                }
            }
        }
        if !is_msgrmsg {
            let mut l_lastQuotedLine_0 = None;
            let mut hasQuotedHeadline = 0;
            for l in l_first..l_last {
                let line = lines[l];
                if is_plain_quote(line) {
                    l_lastQuotedLine_0 = Some(l)
                } else if !is_empty_line(line) {
                    if is_quoted_headline(line)
                        && 0 == hasQuotedHeadline
                        && l_lastQuotedLine_0.is_none()
                    {
                        hasQuotedHeadline = 1i32
                    } else {
                        /* non-quoting line found */
                        break;
                    }
                }
            }
            if l_lastQuotedLine_0.is_some() {
                l_first = l_lastQuotedLine_0.unwrap() + 1;
                is_cut_at_begin = true
            }
        }
        /* re-create buffer from the remaining lines */
        let mut ret = String::new();
        if is_cut_at_begin {
            ret += "[...]";
        }
        /* we write empty lines only in case and non-empty line follows */
        let mut pending_linebreaks: libc::c_int = 0i32;
        let mut content_lines_added: libc::c_int = 0i32;
        for l in l_first..l_last {
            let line = lines[l];
            if is_empty_line(line) {
                pending_linebreaks += 1
            } else {
                if 0 != content_lines_added {
                    if pending_linebreaks > 2i32 {
                        pending_linebreaks = 2i32
                    }
                    while 0 != pending_linebreaks {
                        ret += "\n";
                        pending_linebreaks -= 1
                    }
                }
                // the incoming message might contain invalid UTF8
                ret += line;
                content_lines_added += 1;
                pending_linebreaks = 1i32
            }
        }
        if is_cut_at_end && (!is_cut_at_begin || 0 != content_lines_added) {
            ret += " [...]";
        }

        ret
    }
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
    buf.starts_with(">")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplify_trim() {
        let mut simplify = Simplify::new();
        let html = "\r\r\nline1<br>\r\n\r\n\r\rline2\n\r";
        let plain = simplify.simplify(html, true, false);

        assert_eq!(plain, "line1\nline2");
    }

    #[test]
    fn test_simplify_parse_href() {
        let mut simplify = Simplify::new();
        let html = "<a href=url>text</a";
        let plain = simplify.simplify(html, true, false);

        assert_eq!(plain, "[text](url)");
    }

    #[test]
    fn test_simplify_bold_text() {
        let mut simplify = Simplify::new();
        let html = "<!DOCTYPE name [<!DOCTYPE ...>]><!-- comment -->text <b><?php echo ... ?>bold</b><![CDATA[<>]]>";
        let plain = simplify.simplify(html, true, false);

        assert_eq!(plain, "text *bold*<>");
    }

    #[test]
    fn test_simplify_html_encoded() {
        let mut simplify = Simplify::new();
        let html =
                "&lt;&gt;&quot;&apos;&amp; &auml;&Auml;&ouml;&Ouml;&uuml;&Uuml;&szlig; foo&AElig;&ccedil;&Ccedil; &diams;&lrm;&rlm;&zwnj;&noent;&zwj;";

        let plain = simplify.simplify(html, true, false);

        assert_eq!(
            plain,
            "<>\"\'& äÄöÖüÜß fooÆçÇ \u{2666}\u{200e}\u{200f}\u{200c}&noent;\u{200d}"
        );
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
