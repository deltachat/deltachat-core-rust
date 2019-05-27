use crate::context::Context;

/* yes: uppercase */
/* library private: key-history */
pub fn dc_add_to_keyhistory(
    _context: &Context,
    _rfc724_mid: *const libc::c_char,
    _sending_time: u64,
    _addr: *const libc::c_char,
    _fingerprint: *const libc::c_char,
) {

}
