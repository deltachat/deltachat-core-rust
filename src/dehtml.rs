//! De-HTML.
//!
//! A module to remove HTML tags from the email text

use std::io::BufRead;

use once_cell::sync::Lazy;
use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText},
    Reader,
};

static LINE_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"(\r?\n)+").unwrap());

struct Dehtml {
    strbuilder: String,
    add_text: AddText,
    last_href: Option<String>,
    /// GMX wraps a quote in `<div name="quote">`. After a `<div name="quote">`, this count is
    /// increased at each `<div>` and decreased at each `</div>`. This way we know when the quote ends.
    /// If this is > `0`, then we are inside a `<div name="quote">`
    divs_since_quote_div: u32,
    /// Everything between <div name="quote"> and <div name="quoted-content"> is usually metadata
    /// If this is > `0`, then we are inside a `<div name="quoted-content">`.
    divs_since_quoted_content_div: u32,
    /// All-Inkl just puts the quote into `<blockquote> </blockquote>`. This count is
    /// increased at each `<blockquote>` and decreased at each `</blockquote>`.
    blockquotes_since_blockquote: u32,
}

impl Dehtml {
    fn line_prefix(&self) -> &str {
        if self.divs_since_quoted_content_div > 0 || self.blockquotes_since_blockquote > 0 {
            "> "
        } else {
            ""
        }
    }
    fn append_prefix(&self, line_end: &str) -> String {
        // line_end is e.g. "\n\n". We add "> " if necessary.
        line_end.to_string() + self.line_prefix()
    }
    fn get_add_text(&self) -> AddText {
        if self.divs_since_quote_div > 0 && self.divs_since_quoted_content_div == 0 {
            AddText::No // Everything between <div name="quoted"> and <div name="quoted_content"> is metadata which we don't want
        } else {
            self.add_text
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum AddText {
    No,
    YesRemoveLineEnds,
    YesPreserveLineEnds,
}

// dehtml() returns way too many newlines; however, an optimisation on this issue is not needed as
// the newlines are typically removed in further processing by the caller
pub fn dehtml(buf: &str) -> Option<String> {
    let s = dehtml_quick_xml(buf);
    if !s.trim().is_empty() {
        return Some(s);
    }
    let s = dehtml_manually(buf);
    if !s.trim().is_empty() {
        return Some(s);
    }
    None
}

fn dehtml_quick_xml(buf: &str) -> String {
    let buf = buf.trim().trim_start_matches("<!doctype html>");

    let mut dehtml = Dehtml {
        strbuilder: String::with_capacity(buf.len()),
        add_text: AddText::YesRemoveLineEnds,
        last_href: None,
        divs_since_quote_div: 0,
        divs_since_quoted_content_div: 0,
        blockquotes_since_blockquote: 0,
    };

    let mut reader = quick_xml::Reader::from_str(buf);
    reader.check_end_names(false);

    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                dehtml_starttag_cb(e, &mut dehtml, &reader)
            }
            Ok(quick_xml::events::Event::End(ref e)) => dehtml_endtag_cb(e, &mut dehtml),
            Ok(quick_xml::events::Event::Text(ref e)) => dehtml_text_cb(e, &mut dehtml),
            Ok(quick_xml::events::Event::CData(ref e)) => dehtml_cdata_cb(e, &mut dehtml),
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                // Handle empty tags as a start tag immediately followed by end tag.
                // For example, `<p/>` is treated as `<p></p>`.
                dehtml_starttag_cb(e, &mut dehtml, &reader);
                dehtml_endtag_cb(&BytesEnd::borrowed(e.name()), &mut dehtml);
            }
            Err(e) => {
                eprintln!(
                    "Parse html error: Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                );
            }
            Ok(quick_xml::events::Event::Eof) => break,
            _ => (),
        }
        buf.clear();
    }

    dehtml.strbuilder
}

fn dehtml_text_cb(event: &BytesText, dehtml: &mut Dehtml) {
    if dehtml.get_add_text() == AddText::YesPreserveLineEnds
        || dehtml.get_add_text() == AddText::YesRemoveLineEnds
    {
        let last_added = escaper::decode_html_buf_sloppy(event.escaped()).unwrap_or_default();

        if dehtml.get_add_text() == AddText::YesRemoveLineEnds {
            dehtml.strbuilder += LINE_RE.replace_all(&last_added, "\r").as_ref();
        } else if !dehtml.line_prefix().is_empty() {
            let l = dehtml.append_prefix("\n");
            dehtml.strbuilder += LINE_RE.replace_all(&last_added, l.as_str()).as_ref();
        } else {
            dehtml.strbuilder += &last_added;
        }
    }
}

fn dehtml_cdata_cb(event: &BytesText, dehtml: &mut Dehtml) {
    if dehtml.get_add_text() == AddText::YesPreserveLineEnds
        || dehtml.get_add_text() == AddText::YesRemoveLineEnds
    {
        let last_added = escaper::decode_html_buf_sloppy(event.escaped()).unwrap_or_default();

        if dehtml.get_add_text() == AddText::YesRemoveLineEnds {
            dehtml.strbuilder += LINE_RE.replace_all(&last_added, "\r").as_ref();
        } else if !dehtml.line_prefix().is_empty() {
            let l = dehtml.append_prefix("\n");
            dehtml.strbuilder += LINE_RE.replace_all(&last_added, l.as_str()).as_ref();
        } else {
            dehtml.strbuilder += &last_added;
        }
    }
}

fn dehtml_endtag_cb(event: &BytesEnd, dehtml: &mut Dehtml) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    match tag.as_str() {
        "p" | "table" | "td" | "style" | "script" | "title" | "pre" => {
            dehtml.strbuilder += &dehtml.append_prefix("\n\n");
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "div" => {
            pop_tag(&mut dehtml.divs_since_quote_div);
            pop_tag(&mut dehtml.divs_since_quoted_content_div);

            dehtml.strbuilder += &dehtml.append_prefix("\n\n");
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "a" => {
            if let Some(ref last_href) = dehtml.last_href.take() {
                dehtml.strbuilder += "](";
                dehtml.strbuilder += last_href;
                dehtml.strbuilder += ")";
            }
        }
        "b" | "strong" => {
            if dehtml.get_add_text() != AddText::No {
                dehtml.strbuilder += "*";
            }
        }
        "i" | "em" => {
            if dehtml.get_add_text() != AddText::No {
                dehtml.strbuilder += "_";
            }
        }
        "blockquote" => pop_tag(&mut dehtml.blockquotes_since_blockquote),
        _ => {}
    }
}

fn dehtml_starttag_cb<B: std::io::BufRead>(
    event: &BytesStart,
    dehtml: &mut Dehtml,
    reader: &quick_xml::Reader<B>,
) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    match tag.as_str() {
        "p" | "table" | "td" => {
            dehtml.strbuilder += &dehtml.append_prefix("\n\n");
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        #[rustfmt::skip]
        "div" => {
            maybe_push_tag(event, reader, "quote", &mut dehtml.divs_since_quote_div);
            maybe_push_tag(event, reader, "quoted-content", &mut dehtml.divs_since_quoted_content_div);

            dehtml.strbuilder += &dehtml.append_prefix("\n\n");
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "br" => {
            dehtml.strbuilder += &dehtml.append_prefix("\n");
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "style" | "script" | "title" => {
            dehtml.add_text = AddText::No;
        }
        "pre" => {
            dehtml.strbuilder += &dehtml.append_prefix("\n\n");
            dehtml.add_text = AddText::YesPreserveLineEnds;
        }
        "a" => {
            if let Some(href) = event
                .html_attributes()
                .filter_map(|attr| attr.ok())
                .find(|attr| String::from_utf8_lossy(attr.key).trim().to_lowercase() == "href")
            {
                let href = href
                    .unescape_and_decode_value(reader)
                    .unwrap_or_default()
                    .to_lowercase();

                if !href.is_empty() {
                    dehtml.last_href = Some(href);
                    dehtml.strbuilder += "[";
                }
            }
        }
        "b" | "strong" => {
            if dehtml.get_add_text() != AddText::No {
                dehtml.strbuilder += "*";
            }
        }
        "i" | "em" => {
            if dehtml.get_add_text() != AddText::No {
                dehtml.strbuilder += "_";
            }
        }
        "blockquote" => dehtml.blockquotes_since_blockquote += 1,
        _ => {}
    }
}

/// In order to know when a specific tag is closed, we need to count the opening and closing tags.
/// The `counts`s are stored in the `Dehtml` struct.
fn pop_tag(count: &mut u32) {
    if *count > 0 {
        *count -= 1;
    }
}

/// In order to know when a specific tag is closed, we need to count the opening and closing tags.
/// The `counts`s are stored in the `Dehtml` struct.
fn maybe_push_tag(
    event: &BytesStart,
    reader: &Reader<impl BufRead>,
    tag_name: &str,
    count: &mut u32,
) {
    if *count > 0 || tag_contains_attr(event, reader, tag_name) {
        *count += 1;
    }
}

fn tag_contains_attr(event: &BytesStart, reader: &Reader<impl BufRead>, name: &str) -> bool {
    event.attributes().any(|r| {
        r.map(|a| {
            a.unescape_and_decode_value(reader)
                .map(|v| v == name)
                .unwrap_or(false)
        })
        .unwrap_or(false)
    })
}

pub fn dehtml_manually(buf: &str) -> String {
    // Just strip out everything between "<" and ">"
    let mut strbuilder = String::new();
    let mut show_next_chars = true;
    for c in buf.chars() {
        match c {
            '<' => show_next_chars = false,
            '>' => show_next_chars = true,
            _ => {
                if show_next_chars {
                    strbuilder.push(c)
                }
            }
        }
    }
    strbuilder
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simplify::simplify;

    #[test]
    fn test_dehtml() {
        let cases = vec![
            (
                "<a href='https://example.com'> Foo </a>",
                "[ Foo ](https://example.com)",
            ),
            ("<b> bar </b>", "* bar *"),
            ("<i>foo</i>", "_foo_"),
            ("<b> bar <i> foo", "* bar _ foo"),
            ("&amp; bar", "& bar"),
            // Despite missing ', this should be shown:
            ("<a href='/foo.png>Hi</a> ", "Hi "),
            (
                "<a href='https://get.delta.chat/'/>",
                "[](https://get.delta.chat/)",
            ),
            ("<!doctype html>\n<b>fat text</b>", "*fat text*"),
            // Invalid html (at least DC should show the text if the html is invalid):
            ("<!some invalid html code>\n<b>some text</b>", "some text"),
        ];
        for (input, output) in cases {
            assert_eq!(simplify(dehtml(input).unwrap(), true).0, output);
        }
        let none_cases = vec!["<html> </html>", ""];
        for input in none_cases {
            assert_eq!(dehtml(input), None);
        }
    }

    #[test]
    fn test_dehtml_parse_br() {
        let html = "\r\r\nline1<br>\r\n\r\n\r\rline2<br/>line3\n\r";
        let plain = dehtml(html).unwrap();

        assert_eq!(plain, "line1\n\r\r\rline2\nline3");
    }

    #[test]
    fn test_dehtml_parse_href() {
        let html = "<a href=url>text</a";
        let plain = dehtml(html).unwrap();

        assert_eq!(plain, "[text](url)");
    }

    #[test]
    fn test_dehtml_bold_text() {
        let html = "<!DOCTYPE name [<!DOCTYPE ...>]><!-- comment -->text <b><?php echo ... ?>bold</b><![CDATA[<>]]>";
        let plain = dehtml(html).unwrap();

        assert_eq!(plain, "text *bold*<>");
    }

    #[test]
    fn test_dehtml_html_encoded() {
        let html =
                "&lt;&gt;&quot;&apos;&amp; &auml;&Auml;&ouml;&Ouml;&uuml;&Uuml;&szlig; foo&AElig;&ccedil;&Ccedil; &diams;&lrm;&rlm;&zwnj;&noent;&zwj;";

        let plain = dehtml(html).unwrap();

        assert_eq!(
            plain,
            "<>\"\'& äÄöÖüÜß fooÆçÇ \u{2666}\u{200e}\u{200f}\u{200c}&noent;\u{200d}"
        );
    }

    #[test]
    fn test_unclosed_tags() {
        let input = r##"
        <!DOCTYPE HTML PUBLIC '-//W3C//DTD HTML 4.01 Transitional//EN'
        'http://www.w3.org/TR/html4/loose.dtd'>
        <html>
        <head>
        <title>Hi</title>
        <meta http-equiv='Content-Type' content='text/html; charset=iso-8859-1'>						
        </head>
        <body>
        lots of text
        </body>
        </html>
        "##;
        let txt = dehtml(input).unwrap();
        assert_eq!(txt.trim(), "lots of text");
    }

    #[test]
    fn test_pre_tag() {
        let input = "<html><pre>\ntwo\nlines\n</pre></html>";
        let txt = dehtml(input).unwrap();
        assert_eq!(txt.trim(), "two\nlines");
    }

    #[async_std::test]
    async fn test_quote_div() {
        let input = include_str!("../test-data/message/gmx-quote-body.eml");
        let dehtml = dehtml(input).unwrap();
        println!("{}", dehtml);
        let (msg, forwarded, cut, top_quote, footer) = simplify(dehtml, false);
        assert_eq!(msg, "Test");
        assert_eq!(forwarded, false);
        assert_eq!(cut, false);
        assert_eq!(top_quote.as_deref(), Some("test"));
        assert_eq!(footer, None);
    }
}
