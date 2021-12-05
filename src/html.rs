//! # Get message as HTML.
//!
//! Use `Message.has_html()` to check if the UI shall render a
//! corresponding button and `MsgId.get_html()` to get the full message.
//!
//! Even when the original mime-message is not HTML,
//! `MsgId.get_html()` will return HTML -
//! this allows nice quoting, handling linebreaks properly etc.

use futures::future::FutureExt;
use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use lettre_email::mime::{self, Mime};

use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::message::{Message, MsgId};
use crate::mimeparser::parse_message_id;
use crate::param::Param::SendHtml;
use crate::plaintext::PlainText;
use crate::{context::Context, message};
use lettre_email::PartBuilder;
use mailparse::ParsedContentType;

impl Message {
    /// Check if the message can be retrieved as HTML.
    /// Typically, this is the case, when the mime structure of a Message is modified,
    /// meaning that some text is cut or the original message
    /// is in HTML and `simplify()` may hide some maybe important information.
    /// The corresponding ffi-function is `dc_msg_has_html()`.
    /// To get the HTML-code of the message, use `MsgId.get_html()`.
    pub fn has_html(&self) -> bool {
        self.mime_modified
    }

    /// Set HTML-part part of a message that is about to be sent.
    /// The HTML-part is written to the database before sending and
    /// used as the `text/html` part in the MIME-structure.
    ///
    /// Received HTML parts are handled differently,
    /// they are saved together with the whole MIME-structure
    /// in `mime_headers` and the HTML-part is extracted using `MsgId::get_html()`.
    /// (To underline this asynchronicity, we are using the wording "SendHtml")
    pub fn set_html(&mut self, html: Option<String>) {
        if let Some(html) = html {
            self.param.set(SendHtml, html);
            self.mime_modified = true;
        } else {
            self.param.remove(SendHtml);
            self.mime_modified = false;
        }
    }
}

/// Type defining a rough mime-type.
/// This is mainly useful on iterating
/// to decide whether a mime-part has subtypes.
enum MimeMultipartType {
    Multiple,
    Single,
    Message,
}

/// Function takes a content type from a ParsedMail structure
/// and checks and returns the rough mime-type.
async fn get_mime_multipart_type(ctype: &ParsedContentType) -> MimeMultipartType {
    let mimetype = ctype.mimetype.to_lowercase();
    if mimetype.starts_with("multipart") && ctype.params.get("boundary").is_some() {
        MimeMultipartType::Multiple
    } else if mimetype == "message/rfc822" {
        MimeMultipartType::Message
    } else {
        MimeMultipartType::Single
    }
}

/// HtmlMsgParser converts a mime-message to HTML.
#[derive(Debug)]
struct HtmlMsgParser {
    pub html: String,
    pub plain: Option<PlainText>,
}

impl HtmlMsgParser {
    /// Function takes a raw mime-message string,
    /// searches for the main-text part
    /// and returns that as parser.html
    pub async fn from_bytes(context: &Context, rawmime: &[u8]) -> Result<Self> {
        let mut parser = HtmlMsgParser {
            html: "".to_string(),
            plain: None,
        };

        let parsedmail = mailparse::parse_mail(rawmime)?;

        parser.collect_texts_recursive(context, &parsedmail).await?;

        if parser.html.is_empty() {
            if let Some(plain) = &parser.plain {
                parser.html = plain.to_html().await;
            }
        } else {
            parser.cid_to_data_recursive(context, &parsedmail).await?;
        }

        Ok(parser)
    }

    /// Function iterates over all mime-parts
    /// and searches for text/plain and text/html parts and saves the
    /// first one found.
    /// in the corresponding structure fields.
    ///
    /// Usually, there is at most one plain-text and one HTML-text part,
    /// multiple plain-text parts might be used for mailinglist-footers,
    /// therefore we use the first one.
    fn collect_texts_recursive<'a>(
        &'a mut self,
        context: &'a Context,
        mail: &'a mailparse::ParsedMail<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a + Send>> {
        // Boxed future to deal with recursion
        async move {
            match get_mime_multipart_type(&mail.ctype).await {
                MimeMultipartType::Multiple => {
                    for cur_data in mail.subparts.iter() {
                        self.collect_texts_recursive(context, cur_data).await?
                    }
                    Ok(())
                }
                MimeMultipartType::Message => {
                    let raw = mail.get_body_raw()?;
                    if raw.is_empty() {
                        return Ok(());
                    }
                    let mail = mailparse::parse_mail(&raw).unwrap();
                    self.collect_texts_recursive(context, &mail).await
                }
                MimeMultipartType::Single => {
                    let mimetype = mail.ctype.mimetype.parse::<Mime>()?;
                    if mimetype == mime::TEXT_HTML {
                        if self.html.is_empty() {
                            if let Ok(decoded_data) = mail.get_body() {
                                self.html = decoded_data;
                            }
                        }
                    } else if mimetype == mime::TEXT_PLAIN && self.plain.is_none() {
                        if let Ok(decoded_data) = mail.get_body() {
                            self.plain = Some(PlainText {
                                text: decoded_data,
                                flowed: if let Some(format) = mail.ctype.params.get("format") {
                                    format.as_str().to_ascii_lowercase() == "flowed"
                                } else {
                                    false
                                },
                                delsp: if let Some(delsp) = mail.ctype.params.get("delsp") {
                                    delsp.as_str().to_ascii_lowercase() == "yes"
                                } else {
                                    false
                                },
                            });
                        }
                    }
                    Ok(())
                }
            }
        }
        .boxed()
    }

    /// Replace cid:-protocol by the data:-protocol where appropriate.
    /// This allows the final html-file to be self-contained.
    fn cid_to_data_recursive<'a>(
        &'a mut self,
        context: &'a Context,
        mail: &'a mailparse::ParsedMail<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a + Send>> {
        // Boxed future to deal with recursion
        async move {
            match get_mime_multipart_type(&mail.ctype).await {
                MimeMultipartType::Multiple => {
                    for cur_data in mail.subparts.iter() {
                        self.cid_to_data_recursive(context, cur_data).await?;
                    }
                    Ok(())
                }
                MimeMultipartType::Message => {
                    let raw = mail.get_body_raw()?;
                    if raw.is_empty() {
                        return Ok(());
                    }
                    let mail = mailparse::parse_mail(&raw).unwrap();
                    self.cid_to_data_recursive(context, &mail).await
                }
                MimeMultipartType::Single => {
                    let mimetype = mail.ctype.mimetype.parse::<Mime>()?;
                    if mimetype.type_() == mime::IMAGE {
                        if let Some(cid) = mail.headers.get_header_value(HeaderDef::ContentId) {
                            if let Ok(cid) = parse_message_id(&cid) {
                                if let Ok(replacement) = mimepart_to_data_url(mail).await {
                                    let re_string = format!(
                                        "(<img[^>]*src[^>]*=[^>]*)(cid:{})([^>]*>)",
                                        regex::escape(&cid)
                                    );
                                    match regex::Regex::new(&re_string) {
                                        Ok(re) => {
                                            self.html = re
                                                .replace_all(
                                                    &*self.html,
                                                    format!("${{1}}{}${{3}}", replacement).as_str(),
                                                )
                                                .as_ref()
                                                .to_string()
                                        }
                                        Err(e) => warn!(
                                            context,
                                            "Cannot create regex for cid: {} throws {}",
                                            re_string,
                                            e
                                        ),
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                }
            }
        }
        .boxed()
    }
}

/// Convert a mime part to a data: url as defined in [RFC 2397](https://tools.ietf.org/html/rfc2397).
async fn mimepart_to_data_url(mail: &mailparse::ParsedMail<'_>) -> Result<String> {
    let data = mail.get_body_raw()?;
    let data = base64::encode(&data);
    Ok(format!("data:{};base64,{}", mail.ctype.mimetype, data))
}

impl MsgId {
    /// Get HTML from a message-id.
    /// This requires `mime_headers` field to be set for the message;
    /// this is the case at least when `Message.has_html()` returns true
    /// (we do not save raw mime unconditionally in the database to save space).
    /// The corresponding ffi-function is `dc_get_msg_html()`.
    pub async fn get_html(self, context: &Context) -> Result<Option<String>> {
        let rawmime = message::get_mime_headers(context, self).await?;

        if !rawmime.is_empty() {
            match HtmlMsgParser::from_bytes(context, &rawmime).await {
                Err(err) => {
                    warn!(context, "get_html: parser error: {}", err);
                    Ok(None)
                }
                Ok(parser) => Ok(Some(parser.html)),
            }
        } else {
            warn!(context, "get_html: no mime for {}", self);
            Ok(None)
        }
    }
}

/// Wraps HTML text into a new text/html mimepart structure.
///
/// Used on forwarding messages to avoid leaking the original mime structure
/// and also to avoid sending too much, maybe large data.
pub async fn new_html_mimepart(html: String) -> PartBuilder {
    PartBuilder::new()
        .content_type(&"text/html; charset=utf-8".parse::<mime::Mime>().unwrap())
        .body(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat;
    use crate::chat::forward_msgs;
    use crate::config::Config;
    use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
    use crate::dc_receive_imf::dc_receive_imf;
    use crate::message::MessengerMessage;
    use crate::test_utils::TestContext;

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
        assert!(parser.plain.unwrap().flowed);
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

        // on windows, `\r\n` linends are returned from mimeparser,
        // however, rust multiline-strings use just `\n`;
        // therefore, we just remove `\r` before comparison.
        assert_eq!(
            parser.html.replace("\r", ""),
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
            parser.html.replace("\r", ""), // see comment in test_htmlparse_html()
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
            parser.html.replace("\r", ""), // see comment in test_htmlparse_html()
            r##"<html>
  <p>
    this is <b>html</b>
  </p>
</html>

"##
        );
    }

    #[async_std::test]
    async fn test_htmlparse_apple_cid_jpg() {
        // load raw mime html-data with related image-part (cid:)
        // and make sure, Content-Id has angle-brackets that are removed correctly.
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/apple_cid_jpg.eml");
        let test = String::from_utf8_lossy(raw);
        assert!(test.contains("Content-Id: <8AE052EF-BC90-486F-BB78-58D3590308EC@fritz.box>"));
        assert!(test.contains("cid:8AE052EF-BC90-486F-BB78-58D3590308EC@fritz.box"));
        assert!(test.find("data:").is_none());

        // parsing converts cid: to data:
        let parser = HtmlMsgParser::from_bytes(&t.ctx, raw).await.unwrap();
        assert!(parser.html.contains("<html>"));
        assert!(!parser.html.contains("Content-Id:"));
        assert!(parser.html.contains("data:image/jpeg;base64,/9j/4AAQ"));
        assert!(!parser.html.contains("cid:"));
    }

    #[async_std::test]
    async fn test_get_html_invalid_msgid() {
        let t = TestContext::new().await;
        let msg_id = MsgId::new(100);
        assert!(msg_id.get_html(&t).await.is_err())
    }

    #[async_std::test]
    async fn test_html_forwarding() {
        // alice receives a non-delta html-message
        let alice = TestContext::new_alice().await;
        alice.set_config(Config::ShowEmails, Some("2")).await.ok();
        let chat = alice
            .create_chat_with_contact("", "sender@testrun.org")
            .await;
        let raw = include_bytes!("../test-data/message/text_alt_plain_html.eml");
        dc_receive_imf(&alice, raw, "INBOX", 1, false)
            .await
            .unwrap();
        let msg = alice.get_last_msg_in(chat.get_id()).await;
        assert_ne!(msg.get_from_id(), DC_CONTACT_ID_SELF);
        assert_eq!(msg.is_dc_message, MessengerMessage::No);
        assert!(!msg.is_forwarded());
        assert!(msg.get_text().unwrap().contains("this is plain"));
        assert!(msg.has_html());
        let html = msg.get_id().get_html(&alice).await.unwrap().unwrap();
        assert!(html.contains("this is <b>html</b>"));

        // alice: create chat with bob and forward received html-message there
        let chat = alice.create_chat_with_contact("", "bob@example.net").await;
        forward_msgs(&alice, &[msg.get_id()], chat.get_id())
            .await
            .unwrap();
        let msg = alice.get_last_msg_in(chat.get_id()).await;
        assert_eq!(msg.get_from_id(), DC_CONTACT_ID_SELF);
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert!(msg.is_forwarded());
        assert!(msg.get_text().unwrap().contains("this is plain"));
        assert!(msg.has_html());
        let html = msg.get_id().get_html(&alice).await.unwrap().unwrap();
        assert!(html.contains("this is <b>html</b>"));

        // bob: check that bob also got the html-part of the forwarded message
        let bob = TestContext::new_bob().await;
        let chat = bob.create_chat_with_contact("", "alice@example.org").await;
        bob.recv_msg(&alice.pop_sent_msg().await).await;
        let msg = bob.get_last_msg_in(chat.get_id()).await;
        assert_ne!(msg.get_from_id(), DC_CONTACT_ID_SELF);
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert!(msg.is_forwarded());
        assert!(msg.get_text().unwrap().contains("this is plain"));
        assert!(msg.has_html());
        let html = msg.get_id().get_html(&bob).await.unwrap().unwrap();
        assert!(html.contains("this is <b>html</b>"));
    }

    #[async_std::test]
    async fn test_html_forwarding_encrypted() {
        // Alice receives a non-delta html-message
        // (`ShowEmails=1` lets Alice actually receive non-delta messages for known contacts,
        // the contact is marked as known by creating a chat using `chat_with_contact()`)
        let alice = TestContext::new_alice().await;
        alice.set_config(Config::ShowEmails, Some("1")).await.ok();
        let chat = alice
            .create_chat_with_contact("", "sender@testrun.org")
            .await;
        let raw = include_bytes!("../test-data/message/text_alt_plain_html.eml");
        dc_receive_imf(&alice, raw, "INBOX", 1, false)
            .await
            .unwrap();
        let msg = alice.get_last_msg_in(chat.get_id()).await;

        // forward the message to saved-messages,
        // this will encrypt the message as new_alice() has set up keys
        let chat = alice.get_self_chat().await;
        forward_msgs(&alice, &[msg.get_id()], chat.get_id())
            .await
            .unwrap();
        let msg = alice.pop_sent_msg().await;

        // receive the message on another device
        let alice = TestContext::new_alice().await;
        assert_eq!(alice.get_config_int(Config::ShowEmails).await.unwrap(), 0); // set to "1" above, make sure it is another db
        alice.recv_msg(&msg).await;
        let chat = alice.get_self_chat().await;
        let msg = alice.get_last_msg_in(chat.get_id()).await;
        assert_eq!(msg.get_from_id(), DC_CONTACT_ID_SELF);
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert!(msg.get_showpadlock());
        assert!(msg.is_forwarded());
        assert!(msg.get_text().unwrap().contains("this is plain"));
        assert!(msg.has_html());
        let html = msg.get_id().get_html(&alice).await.unwrap().unwrap();
        assert!(html.contains("this is <b>html</b>"));
    }

    #[async_std::test]
    async fn test_set_html() {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // alice sends a message with html-part to bob
        let chat_id = alice.create_chat(&bob).await.id;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("plain text".to_string()));
        msg.set_html(Some("<b>html</b> text".to_string()));
        assert!(msg.mime_modified);
        chat::send_msg(&alice, chat_id, &mut msg).await.unwrap();

        // check the message is written correctly to alice's db
        let msg = alice.get_last_msg_in(chat_id).await;
        assert_eq!(msg.get_text(), Some("plain text".to_string()));
        assert!(!msg.is_forwarded());
        assert!(msg.mime_modified);
        let html = msg.get_id().get_html(&alice).await.unwrap().unwrap();
        assert!(html.contains("<b>html</b> text"));

        // let bob receive the message
        let chat_id = bob.create_chat(&alice).await.id;
        bob.recv_msg(&alice.pop_sent_msg().await).await;
        let msg = bob.get_last_msg_in(chat_id).await;
        assert_eq!(msg.get_text(), Some("plain text".to_string()));
        assert!(!msg.is_forwarded());
        assert!(msg.mime_modified);
        let html = msg.get_id().get_html(&bob).await.unwrap().unwrap();
        assert!(html.contains("<b>html</b> text"));
    }

    #[async_std::test]
    async fn test_cp1252_html() -> Result<()> {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("2")).await?;
        dc_receive_imf(
            &t,
            include_bytes!("../test-data/message/cp1252-html.eml"),
            "INBOX",
            0,
            false,
        )
        .await?;
        let msg = t.get_last_msg().await;
        assert_eq!(msg.viewtype, Viewtype::Text);
        assert!(msg.text.as_ref().unwrap().contains("foo bar ä ö ü ß"));
        assert!(msg.has_html());
        let html = msg.get_id().get_html(&t).await?.unwrap();
        println!("{}", html);
        assert!(html.contains("foo bar ä ö ü ß"));
        Ok(())
    }
}
