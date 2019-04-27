use libc;

use crate::dc_context::dc_context_t;
use crate::types::*;

/* yes: uppercase */
/* library private: key-history */
pub unsafe fn dc_add_to_keyhistory(
    _context: *mut dc_context_t,
    _rfc724_mid: *const libc::c_char,
    _sending_time: time_t,
    _addr: *const libc::c_char,
    _fingerprint: *const libc::c_char,
) {

}
