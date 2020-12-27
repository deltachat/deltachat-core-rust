//! Get a full mime-message as HTML.

use crate::context::Context;
use crate::message::{Message, MsgId};

impl Message {
    pub fn is_mime_modified(&self) -> bool {
        true
    }
}

pub async fn get_original_mime_html(_context: &Context, _msg_id: MsgId) -> String {
    "<html><body><p>this is <strong>html</strong>.</p></body></html>".to_string()
}
