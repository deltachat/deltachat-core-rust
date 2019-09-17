use lazy_static::lazy_static;
use quick_xml;
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

// dc_dehtml() returns way too many lineends; however, an optimisation on this issue is not needed as
// the lineends are typically remove in further processing by the caller
pub fn dc_dehtml(buf_terminated: &str) -> String {
    let buf_terminated = buf_terminated.trim();

    if buf_terminated.is_empty() {
        return "".into();
    }

    let mut dehtml = Dehtml {
        strbuilder: String::with_capacity(buf_terminated.len()),
        add_text: AddText::YesRemoveLineEnds,
        last_href: None,
    };

    let mut reader = quick_xml::Reader::from_str(buf_terminated);

    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                dehtml_starttag_cb(e, &mut dehtml, &reader)
            }
            Ok(quick_xml::events::Event::End(ref e)) => dehtml_endtag_cb(e, &mut dehtml),
            Ok(quick_xml::events::Event::Text(ref e)) => dehtml_text_cb(e, &mut dehtml),
            Ok(quick_xml::events::Event::CData(ref e)) => dehtml_cdata_cb(e, &mut dehtml),
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
            if let Some(href) = event.html_attributes().find(|attr| {
                attr.as_ref()
                    .map(|a| String::from_utf8_lossy(a.key).trim().to_lowercase() == "href")
                    .unwrap_or_default()
            }) {
                let href = href
                    .unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_dehtml() {
        let cases = vec![
            (
                "<a href='https://example.com'> Foo </a>",
                "[ Foo ](https://example.com)",
            ),
            ("<img href='/foo.png'>", ""),
            ("<b> bar </b>", "* bar *"),
            ("<b> bar <i> foo", "* bar _ foo"),
            ("&amp; bar", "& bar"),
            // Note missing '
            ("<a href='/foo.png>Hi</a> ", ""),
            ("", ""),
        ];
        for (input, output) in cases {
            assert_eq!(dc_dehtml(input), output);
        }
    }
}
