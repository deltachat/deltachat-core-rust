use crate::dc_dehtml::*;
use crate::dc_tools::*;
use crate::x::*;

#[derive(Copy, Clone)]
pub struct Simplify {
    pub is_forwarded: bool,
    pub is_cut_at_begin: bool,
    pub is_cut_at_end: bool,
}

impl Simplify {
    pub fn new() -> Self {
        Simplify {
            is_forwarded: false,
            is_cut_at_begin: false,
            is_cut_at_end: false,
        }
    }

    /// Simplify and normalise text: Remove quotes, signatures, unnecessary
    /// lineends etc.
    /// The data returned from simplify() must be free()'d when no longer used.
    pub unsafe fn simplify(
        &mut self,
        in_unterminated: *const libc::c_char,
        in_bytes: libc::c_int,
        is_html: bool,
        is_msgrmsg: libc::c_int,
    ) -> *mut libc::c_char {
        if in_bytes <= 0 {
            return "".strdup();
        }

        /* create a copy of the given buffer */
        let mut out: *mut libc::c_char;
        let mut temp: *mut libc::c_char;
        self.is_forwarded = false;
        self.is_cut_at_begin = false;
        self.is_cut_at_end = false;
        out = strndup(
            in_unterminated as *mut libc::c_char,
            in_bytes as libc::c_ulong,
        );
        if out.is_null() {
            return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
        }
        if is_html {
            temp = dc_dehtml(out);
            if !temp.is_null() {
                free(out as *mut libc::c_void);
                out = temp
            }
        }
        dc_remove_cr_chars(out);
        temp = self.simplify_plain_text(out, is_msgrmsg);
        if !temp.is_null() {
            free(out as *mut libc::c_void);
            out = temp
        }
        dc_remove_cr_chars(out);

        out
    }

    /**
     * Simplify Plain Text
     */
    #[allow(non_snake_case)]
    unsafe fn simplify_plain_text(
        &mut self,
        buf_terminated: *const libc::c_char,
        is_msgrmsg: libc::c_int,
    ) -> *mut libc::c_char {
        /* This function ...
        ... removes all text after the line `-- ` (footer mark)
        ... removes full quotes at the beginning and at the end of the text -
            these are all lines starting with the character `>`
        ... remove a non-empty line before the removed quote (contains sth. like "On 2.9.2016, Bjoern wrote:" in different formats and lanugages) */
        /* split the given buffer into lines */
        let lines = dc_split_into_lines(buf_terminated);
        let mut l_first: usize = 0;
        let mut l_last = lines.len();
        let mut line: *mut libc::c_char;
        let mut footer_mark: libc::c_int = 0i32;
        for l in l_first..l_last {
            line = lines[l];
            if strcmp(line, b"-- \x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(line, b"--  \x00" as *const u8 as *const libc::c_char) == 0i32
            {
                footer_mark = 1i32
            }
            if strcmp(line, b"--\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(line, b"---\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(line, b"----\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                footer_mark = 1i32;
                self.is_cut_at_end = true
            }
            if 0 != footer_mark {
                l_last = l;
                /* done */
                break;
            }
        }
        if l_last > l_first + 2 {
            let line0: *mut libc::c_char = lines[l_first];
            let line1: *mut libc::c_char = lines[l_first + 1];
            let line2: *mut libc::c_char = lines[l_first + 2];
            if strcmp(
                line0,
                b"---------- Forwarded message ----------\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                && strncmp(line1, b"From: \x00" as *const u8 as *const libc::c_char, 6) == 0i32
                && *line2.offset(0isize) as libc::c_int == 0i32
            {
                self.is_forwarded = true;
                l_first += 3
            }
        }
        for l in l_first..l_last {
            line = lines[l];
            if strncmp(line, b"-----\x00" as *const u8 as *const libc::c_char, 5) == 0i32
                || strncmp(line, b"_____\x00" as *const u8 as *const libc::c_char, 5) == 0i32
                || strncmp(line, b"=====\x00" as *const u8 as *const libc::c_char, 5) == 0i32
                || strncmp(line, b"*****\x00" as *const u8 as *const libc::c_char, 5) == 0i32
                || strncmp(line, b"~~~~~\x00" as *const u8 as *const libc::c_char, 5) == 0i32
            {
                l_last = l;
                self.is_cut_at_end = true;
                /* done */
                break;
            }
        }
        if 0 == is_msgrmsg {
            let mut l_lastQuotedLine = None;
            for l in (l_first..l_last).rev() {
                line = lines[l];
                if is_plain_quote(line) {
                    l_lastQuotedLine = Some(l)
                } else if !is_empty_line(line) {
                    break;
                }
            }
            if l_lastQuotedLine.is_some() {
                l_last = l_lastQuotedLine.unwrap();
                self.is_cut_at_end = true;
                if l_last > 1 {
                    if is_empty_line(lines[l_last - 1]) {
                        l_last -= 1
                    }
                }
                if l_last > 1 {
                    line = lines[l_last - 1];
                    if is_quoted_headline(line) {
                        l_last -= 1
                    }
                }
            }
        }
        if 0 == is_msgrmsg {
            let mut l_lastQuotedLine_0 = None;
            let mut hasQuotedHeadline = 0;
            for l in l_first..l_last {
                line = lines[l];
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
                self.is_cut_at_begin = true
            }
        }
        /* re-create buffer from the remaining lines */
        let mut ret = String::new();
        if self.is_cut_at_begin {
            ret += "[...]";
        }
        /* we write empty lines only in case and non-empty line follows */
        let mut pending_linebreaks: libc::c_int = 0i32;
        let mut content_lines_added: libc::c_int = 0i32;
        for l in l_first..l_last {
            line = lines[l];
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
                ret += &to_string_lossy(line);
                content_lines_added += 1;
                pending_linebreaks = 1i32
            }
        }
        if self.is_cut_at_end && (!self.is_cut_at_begin || 0 != content_lines_added) {
            ret += " [...]";
        }
        dc_free_splitted_lines(lines);

        ret.strdup()
    }
}

/**
 * Tools
 */
unsafe fn is_empty_line(buf: *const libc::c_char) -> bool {
    /* force unsigned - otherwise the `> ' '` comparison will fail */
    let mut p1: *const libc::c_uchar = buf as *const libc::c_uchar;
    while 0 != *p1 {
        if *p1 as libc::c_int > ' ' as i32 {
            return false;
        }
        p1 = p1.offset(1isize)
    }

    true
}

unsafe fn is_quoted_headline(buf: *const libc::c_char) -> bool {
    /* This function may be called for the line _directly_ before a quote.
    The function checks if the line contains sth. like "On 01.02.2016, xy@z wrote:" in various languages.
    - Currently, we simply check if the last character is a ':'.
    - Checking for the existence of an email address may fail (headlines may show the user's name instead of the address) */
    let buf_len: libc::c_int = strlen(buf) as libc::c_int;
    if buf_len > 80i32 {
        return false;
    }
    if buf_len > 0i32 && *buf.offset((buf_len - 1i32) as isize) as libc::c_int == ':' as i32 {
        return true;
    }

    false
}

unsafe fn is_plain_quote(buf: *const libc::c_char) -> bool {
    if *buf.offset(0isize) as libc::c_int == '>' as i32 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_simplify_trim() {
        unsafe {
            let mut simplify = Simplify::new();
            let html: *const libc::c_char =
                b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
            let plain: *mut libc::c_char =
                simplify.simplify(html, strlen(html) as libc::c_int, true, 0);

            assert_eq!(
                CStr::from_ptr(plain as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "line1\nline2",
            );

            free(plain as *mut libc::c_void);
        }
    }

    #[test]
    fn test_simplify_parse_href() {
        unsafe {
            let mut simplify = Simplify::new();
            let html: *const libc::c_char =
                b"<a href=url>text</a\x00" as *const u8 as *const libc::c_char;
            let plain: *mut libc::c_char =
                simplify.simplify(html, strlen(html) as libc::c_int, true, 0);

            assert_eq!(
                CStr::from_ptr(plain as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "[text](url)",
            );

            free(plain as *mut libc::c_void);
        }
    }

    #[test]
    fn test_simplify_bold_text() {
        unsafe {
            let mut simplify = Simplify::new();
            let html: *const libc::c_char =
                b"<!DOCTYPE name [<!DOCTYPE ...>]><!-- comment -->text <b><?php echo ... ?>bold</b><![CDATA[<>]]>\x00"
                as *const u8 as *const libc::c_char;
            let plain: *mut libc::c_char =
                simplify.simplify(html, strlen(html) as libc::c_int, true, 0);

            assert_eq!(
                CStr::from_ptr(plain as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "text *bold*<>",
            );

            free(plain as *mut libc::c_void);
        }
    }

    #[test]
    fn test_simplify_html_encoded() {
        unsafe {
            let mut simplify = Simplify::new();
            let html: *const libc::c_char =
                b"&lt;&gt;&quot;&apos;&amp; &auml;&Auml;&ouml;&Ouml;&uuml;&Uuml;&szlig; foo&AElig;&ccedil;&Ccedil; &diams;&noent;&lrm;&rlm;&zwnj;&zwj;\x00"
                as *const u8 as *const libc::c_char;
            let plain: *mut libc::c_char =
                simplify.simplify(html, strlen(html) as libc::c_int, true, 0);

            assert_eq!(
                strcmp(plain,
                       b"<>\"\'& \xc3\xa4\xc3\x84\xc3\xb6\xc3\x96\xc3\xbc\xc3\x9c\xc3\x9f foo\xc3\x86\xc3\xa7\xc3\x87 \xe2\x99\xa6&noent;\x00"
                       as *const u8 as *const libc::c_char),
                0,
            );

            free(plain as *mut libc::c_void);
        }
    }
}
