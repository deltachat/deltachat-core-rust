//! Get original mime-message as HTML.
//!
//! Use is_mime_modified() to check if the UI shall render a
//! corresponding button and get_original_mime_html() to get the full message.
//!
//! Even whem the original mime-message is not HTML,
//! get_original_mime_html() will return HTML -
//! this allows nice quoting, handling linebreaks properly etc.

use std::future::Future;
use std::pin::Pin;

use lettre_email::mime::{self, Mime};

use crate::context::Context;
use crate::error::Result;
use crate::message::{Message, MsgId};
use crate::simplify::split_lines;
use once_cell::sync::Lazy;

impl Message {
    pub fn is_mime_modified(&self) -> bool {
        self.mime_modified
    }
}

// HtmlMsgParser converts a mime-message to HTML.
#[derive(Debug)]
pub struct HtmlMsgParser {
    pub html: String,
    pub plain: Option<String>,
    pub format_flowed: bool,
    pub delsp: bool,
}

impl HtmlMsgParser {
    pub async fn from_bytes(context: &Context, rawmime: &[u8]) -> Result<Self> {
        let mut parser = HtmlMsgParser {
            html: "".to_string(),
            plain: None,
            format_flowed: false,
            delsp: false,
        };

        let parsedmail = mailparse::parse_mail(rawmime)?;

        parser.parse_mime_recursive(context, &parsedmail).await?;

        if parser.html.is_empty() {
            if let Some(plain) = parser.plain.clone() {
                parser.html = plain_to_html(&plain, parser.format_flowed, parser.delsp).await;
            }
        }

        Ok(parser)
    }

    fn parse_mime_recursive<'a>(
        &'a mut self,
        context: &'a Context,
        mail: &'a mailparse::ParsedMail<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a + Send>> {
        use futures::future::FutureExt;

        // Boxed future to deal with recursion
        async move {
            enum MimeS {
                Multiple,
                Single,
                Message,
            }

            let mimetype = mail.ctype.mimetype.to_lowercase();

            let m = if mimetype.starts_with("multipart") {
                if mail.ctype.params.get("boundary").is_some() {
                    MimeS::Multiple
                } else {
                    MimeS::Single
                }
            } else if mimetype.starts_with("message") {
                if mimetype == "message/rfc822" {
                    MimeS::Message
                } else {
                    MimeS::Single
                }
            } else {
                MimeS::Single
            };

            match m {
                MimeS::Multiple => self.handle_multiple(context, mail).await,
                MimeS::Message => {
                    let raw = mail.get_body_raw()?;
                    if raw.is_empty() {
                        return Ok(false);
                    }
                    let mail = mailparse::parse_mail(&raw).unwrap();

                    self.parse_mime_recursive(context, &mail).await
                }
                MimeS::Single => self.add_single_part_if_known(context, mail).await,
            }
        }
        .boxed()
    }

    async fn handle_multiple(
        &mut self,
        context: &Context,
        mail: &mailparse::ParsedMail<'_>,
    ) -> Result<bool> {
        let mut any_part_added = false;
        for cur_data in mail.subparts.iter() {
            if self.parse_mime_recursive(context, cur_data).await? {
                any_part_added = true;
            }
        }
        Ok(any_part_added)
    }

    async fn add_single_part_if_known(
        &mut self,
        _context: &Context,
        mail: &mailparse::ParsedMail<'_>,
    ) -> Result<bool> {
        let mimetype = mail.ctype.mimetype.parse::<Mime>()?;
        if mimetype == mime::TEXT_HTML {
            if let Ok(decoded_data) = mail.get_body() {
                self.html = decoded_data;
                return Ok(true);
            }
        } else if mimetype == mime::TEXT_PLAIN {
            if let Ok(decoded_data) = mail.get_body() {
                self.plain = Some(decoded_data);
                self.format_flowed = if let Some(format) = mail.ctype.params.get("format") {
                    format.as_str().to_ascii_lowercase() == "flowed"
                } else {
                    false
                };
                self.delsp = if let Some(delsp) = mail.ctype.params.get("delsp") {
                    delsp.as_str().to_ascii_lowercase() == "yes"
                } else {
                    false
                };
                return Ok(true);
            }
        }
        Ok(false)
    }
}

// convert plain text to html
async fn plain_to_html(plain_utf8: &str, flowed: bool, delsp: bool) -> String {
    static LINKIFY_MAIL_RE: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r#"\b([\w.\-+]+@[\w.\-]+)\b"#).unwrap());

    static LINKIFY_URL_RE: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(r#"\b((http|https|ftp|ftps):[\w.,:;$/@!?&%\-~=#+]+)"#).unwrap()
    });

    let lines = split_lines(&plain_utf8);

    let mut ret =
        "<!DOCTYPE html>\n<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" /></head><body>\n".to_string();

    for line in lines {
        // we need to do html-entity-encoding after linkify, as otherwise encapsulated links
        // as <http://example.org> cannot be handled not handled correctly
        // (they would become &lt;http://example.org&gt; where the trailing &gt; would become a valid url part).
        // to avoid double encoding, we escape our html-entities by \r that must not be used in the string elsewhere.
        let line = line.to_string().replace("\r", "");

        let mut line = LINKIFY_MAIL_RE
            .replace_all(&*line, "\rLTa href=\rQUOTmailto:$1\rQUOT\rGT$1\rLT/a\rGT")
            .as_ref()
            .to_string();

        line = LINKIFY_URL_RE
            .replace_all(&*line, "\rLTa href=\rQUOT$1\rQUOT\rGT$1\rLT/a\rGT")
            .as_ref()
            .to_string();

        // encode html-entities after linkify the raw string
        line = escaper::encode_minimal(&line);

        // make our escaped html-entities real after encoding all others
        line = line.replace("\rLT", "<");
        line = line.replace("\rGT", ">");
        line = line.replace("\rQUOT", "\"");

        // prepare for next line
        if flowed {
            if line.starts_with(' ') {
                line = line[1..].to_string();
            }
            if line.ends_with(' ') {
                if delsp {
                    line.pop();
                }
            } else {
                line += "<br/>\n";
            }
        } else {
            line += "<br/>\n";
        }

        ret += &*line;
    }
    ret += "</body></html>\n";
    ret
}

// Top-level-function to get html from a message-id
pub async fn get_original_mime_html(context: &Context, msg_id: MsgId) -> String {
    let rawmime: Option<String> = context
        .sql
        .query_get_value(
            context,
            "SELECT mime_headers FROM msgs WHERE id=?;",
            paramsv![msg_id],
        )
        .await;

    if let Some(rawmime) = rawmime {
        match HtmlMsgParser::from_bytes(context, rawmime.as_bytes()).await {
            Err(err) => format!("parser error: {}", err),
            Ok(parser) => parser.html,
        }
    } else {
        format!("parser error: no mime for {}", msg_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[async_std::test]
    async fn test_plain_to_html() {
        let html = plain_to_html(
            r##"line 1
line 2
line with https://link-mid-of-line.org and http://link-end-of-line.com/file?foo=bar%20
http://link-at-start-of-line.org
"##,
            false,
            false,
        )
        .await;
        assert_eq!(
            html,
            r##"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
line 1<br/>
line 2<br/>
line with <a href="https://link-mid-of-line.org">https://link-mid-of-line.org</a> and <a href="http://link-end-of-line.com/file?foo=bar%20">http://link-end-of-line.com/file?foo=bar%20</a><br/>
<a href="http://link-at-start-of-line.org">http://link-at-start-of-line.org</a><br/>
<br/>
</body></html>
"##
        );
    }

    #[async_std::test]
    async fn test_plain_to_html_encapsulated() {
        let html = plain_to_html(
            r#"line with <http://encapsulated.link/?foo=_bar> here!"#,
            false,
            false,
        )
        .await;
        assert_eq!(
            html,
            r#"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
line with &lt;<a href="http://encapsulated.link/?foo=_bar">http://encapsulated.link/?foo=_bar</a>&gt; here!<br/>
</body></html>
"#
        );
    }

    #[async_std::test]
    async fn test_plain_to_html_nolink() {
        let html = plain_to_html(r#"line with nohttp://no.link here"#, false, false).await;
        assert_eq!(
            html,
            r#"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
line with nohttp://no.link here<br/>
</body></html>
"#
        );
    }

    #[async_std::test]
    async fn test_plain_to_html_mailto() {
        let html = plain_to_html(
            r#"just an address: foo@bar.org another@one.de"#,
            false,
            false,
        )
        .await;
        assert_eq!(
            html,
            r#"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
just an address: <a href="mailto:foo@bar.org">foo@bar.org</a> <a href="mailto:another@one.de">another@one.de</a><br/>
</body></html>
"#
        );
    }

    #[async_std::test]
    async fn test_htmlparse_plain_unspecified() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_plain_unspecified.eml");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert_eq!(
            parser.html,
            r##"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
This message does not have Content-Type nor Subject.<br/>
<br/>
</body></html>
"##
        );
    }

    #[async_std::test]
    async fn test_htmlparse_plain_iso88591() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_plain_iso88591.eml");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert_eq!(
            parser.html,
            r##"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
message with a non-UTF-8 encoding: äöüßÄÖÜ<br/>
<br/>
</body></html>
"##
        );
    }

    #[async_std::test]
    async fn test_htmlparse_plain_flowed() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_plain_flowed.eml");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert!(parser.format_flowed);
        assert_eq!(
            parser.html,
            r##"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
This line ends with a space and will be merged with the next one due to format=flowed.<br/>
<br/>
This line does not end with a space<br/>
and will be wrapped as usual.<br/>
<br/>
</body></html>
"##
        );
    }

    #[async_std::test]
    async fn test_htmlparse_alt_plain() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_alt_plain.eml");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert_eq!(
            parser.html,
            r##"<!DOCTYPE html>
<html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8" /></head><body>
mime-modified should not be set set as there is no html and no special stuff;<br/>
although not being a delta-message.<br/>
test some special html-characters as &lt; &gt; and &amp; but also &quot; and &#x27; :)<br/>
<br/>
<br/>
</body></html>
"##
        );
    }

    #[async_std::test]
    async fn test_htmlparse_html() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_html.eml");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert_eq!(
            parser.html,
            r##"
<html>
  <p>mime-modified <b>set</b>; simplify is always regarded as lossy.</p>
</html>"##
        );
    }

    #[async_std::test]
    async fn test_htmlparse_alt_html() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_alt_html.eml");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert_eq!(
            parser.html,
            r##"<html>
  <p>mime-modified <b>set</b>; simplify is always regarded as lossy.</p>
</html>

"##
        );
    }

    #[async_std::test]
    async fn test_htmlparse_alt_plain_html() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_alt_plain_html.eml");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert_eq!(
            parser.html,
            r##"<html>
  <p>
    this is <b>html</b>
  </p>
</html>

"##
        );
    }
}
