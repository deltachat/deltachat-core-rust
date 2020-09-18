//! De-HTML
//!
//! A module to remove HTML tags from the email text

use lazy_static::lazy_static;
use quick_xml::events::{BytesEnd, BytesStart, BytesText};

lazy_static! {
    static ref LINE_RE: regex::Regex = regex::Regex::new(r"(\r?\n)+").unwrap();
}

struct Dehtml {
    strbuilder: String,
    add_text: AddText,
    last_href: Option<String>,
}

#[derive(Debug, PartialEq)]
enum AddText {
    No,
    YesRemoveLineEnds,
    YesPreserveLineEnds,
}

// dehtml() returns way too many newlines; however, an optimisation on this issue is not needed as
// the newlines are typically removed in further processing by the caller
pub fn dehtml(buf: &str) -> String {
    let s = dehtml_quick_xml(buf);
    if !s.trim().is_empty() {
        return s;
    }
    let s = dehtml_manually(buf);
    if !s.trim().is_empty() {
        return s;
    }
    buf.to_string()
}

pub fn dehtml_quick_xml(buf: &str) -> String {
    let buf = buf.trim().trim_start_matches("<!doctype html>");

    let mut dehtml = Dehtml {
        strbuilder: String::with_capacity(buf.len()),
        add_text: AddText::YesRemoveLineEnds,
        last_href: None,
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
    if dehtml.add_text == AddText::YesPreserveLineEnds
        || dehtml.add_text == AddText::YesRemoveLineEnds
    {
        let last_added = escaper::decode_html_buf_sloppy(event.escaped()).unwrap_or_default();

        if dehtml.add_text == AddText::YesRemoveLineEnds {
            dehtml.strbuilder += LINE_RE.replace_all(&last_added, "\r").as_ref();
        } else {
            dehtml.strbuilder += &last_added;
        }
    }
}

fn dehtml_cdata_cb(event: &BytesText, dehtml: &mut Dehtml) {
    if dehtml.add_text == AddText::YesPreserveLineEnds
        || dehtml.add_text == AddText::YesRemoveLineEnds
    {
        let last_added = escaper::decode_html_buf_sloppy(event.escaped()).unwrap_or_default();

        if dehtml.add_text == AddText::YesRemoveLineEnds {
            dehtml.strbuilder += LINE_RE.replace_all(&last_added, "\r").as_ref();
        } else {
            dehtml.strbuilder += &last_added;
        }
    }
}

fn dehtml_endtag_cb(event: &BytesEnd, dehtml: &mut Dehtml) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    match tag.as_str() {
        "p" | "div" | "table" | "td" | "style" | "script" | "title" | "pre" => {
            dehtml.strbuilder += "\n\n";
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
            dehtml.strbuilder += "*";
        }
        "i" | "em" => {
            dehtml.strbuilder += "_";
        }
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
        "p" | "div" | "table" | "td" => {
            dehtml.strbuilder += "\n\n";
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "br" => {
            dehtml.strbuilder += "\n";
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "style" | "script" | "title" => {
            dehtml.add_text = AddText::No;
        }
        "pre" => {
            dehtml.strbuilder += "\n\n";
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
            dehtml.strbuilder += "*";
        }
        "i" | "em" => {
            dehtml.strbuilder += "_";
        }
        _ => {}
    }
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
            ("<b> bar <i> foo", "* bar _ foo"),
            ("&amp; bar", "& bar"),
            // Despite missing ', this should be shown:
            ("<a href='/foo.png>Hi</a> ", "Hi "),
            (
                "<a href='https://get.delta.chat/'/>",
                "[](https://get.delta.chat/)",
            ),
            ("", ""),
            ("<!doctype html>\n<b>fat text</b>", "*fat text*"),
            // Invalid html (at least DC should show the text if the html is invalid):
            ("<!some invalid html code>\n<b>some text</b>", "some text"),
            ("<This text is in brackets>", "<This text is in brackets>"),
        ];
        for (input, output) in cases {
            assert_eq!(simplify(dehtml(input), true).0, output);
        }
    }

    #[test]
    fn test_dehtml_parse_br() {
        let html = "\r\r\nline1<br>\r\n\r\n\r\rline2<br/>line3\n\r";
        let plain = dehtml(html);

        assert_eq!(plain, "line1\n\r\r\rline2\nline3");
    }

    #[test]
    fn test_dehtml_parse_href() {
        let html = "<a href=url>text</a";
        let plain = dehtml(html);

        assert_eq!(plain, "[text](url)");
    }

    #[test]
    fn test_dehtml_bold_text() {
        let html = "<!DOCTYPE name [<!DOCTYPE ...>]><!-- comment -->text <b><?php echo ... ?>bold</b><![CDATA[<>]]>";
        let plain = dehtml(html);

        assert_eq!(plain, "text *bold*<>");
    }

    #[test]
    fn test_dehtml_html_encoded() {
        let html =
                "&lt;&gt;&quot;&apos;&amp; &auml;&Auml;&ouml;&Ouml;&uuml;&Uuml;&szlig; foo&AElig;&ccedil;&Ccedil; &diams;&lrm;&rlm;&zwnj;&noent;&zwj;";

        let plain = dehtml(html);

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
        let txt = dehtml(input);
        assert_eq!(txt.trim(), "lots of text");
    }
}
