use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_context::dc_context_t;
use crate::dc_lot::dc_lot_t;
use crate::dc_sqlite3::dc_sqlite3_t;
use crate::types::*;
use crate::x::*;

/* yes: uppercase */
/* library private: key-history */
#[no_mangle]
pub unsafe extern "C" fn dc_add_to_keyhistory(
    mut context: *mut dc_context_t,
    mut rfc724_mid: *const libc::c_char,
    mut sending_time: time_t,
    mut addr: *const libc::c_char,
    mut fingerprint: *const libc::c_char,
) {

}
