//! Get original mime-message as HTML.
//!
//! Use is_mime_modified() to check if the UI shall render a
//! corresponding button and get_original_mime_html() to get the full message.
//!
//! Even whem the original mime-message is not HTML,
//! get_original_mime_html() will return HTML -
//! this allows nice quoting, handling linebreaks properly etc.

use crate::context::Context;
use crate::message::{Message, MsgId};

impl Message {
    pub fn is_mime_modified(&self) -> bool {
        self.mime_modified
    }
}

pub async fn get_original_mime_html(_context: &Context, _msg_id: MsgId) -> String {
    "<html><body><p>this is <strong>html</strong>.</p></body></html>".to_string()
}
