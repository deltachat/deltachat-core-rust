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
}

impl HtmlMsgParser {
    pub async fn from_bytes(context: &Context, rawmime: &[u8]) -> Result<Self> {
        let mut parser = HtmlMsgParser {
            html: "".to_string(),
            plain: None,
        };

        let parsedmail = mailparse::parse_mail(rawmime)?;

        parser.parse_mime_recursive(context, &parsedmail).await?;

        if parser.html.is_empty() {
            if let Some(plain) = parser.plain.clone() {
                parser.html = plain; // TODO: that should be converted to HTML and corresponding tests should be addapted
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
                return Ok(true);
            }
        }
        Ok(false)
    }
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
    async fn test_htmlparse_plain() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/mail_with_cc.txt");
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert_eq!(
            parser.html,
            r##"hi
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
            r##"mime-modified should not be set set as there is no html and no special stuff; although not being a delta-message.

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
