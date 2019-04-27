use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_context::dc_context_t;
use crate::dc_lot::dc_lot_t;
use crate::dc_sqlite3::dc_sqlite3_t;
use crate::types::*;
use crate::x::*;

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
