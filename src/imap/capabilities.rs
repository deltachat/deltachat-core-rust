//! # IMAP capabilities
//!
//! IMAP server capabilities are determined with a `CAPABILITY` command.
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct Capabilities {
    /// True if the server has IDLE capability as defined in
    /// <https://tools.ietf.org/html/rfc2177>
    pub can_idle: bool,

    /// True if the server has NOTIFY capability as defined in
    /// <https://tools.ietf.org/html/rfc5465>
    pub can_notify: bool,

    /// True if the server has MOVE capability as defined in
    /// <https://tools.ietf.org/html/rfc6851>
    pub can_move: bool,

    /// True if the server has QUOTA capability as defined in
    /// <https://tools.ietf.org/html/rfc2087>
    pub can_check_quota: bool,

    /// True if the server has CONDSTORE capability as defined in
    /// <https://tools.ietf.org/html/rfc7162>
    pub can_condstore: bool,

    /// Server ID if the server supports ID capability.
    pub server_id: Option<HashMap<String, String>>,
}
