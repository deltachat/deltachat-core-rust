#[macro_use]
mod macros;

mod c_vec;
mod errors;
mod hash;
mod key;
mod message;
mod public_key;
mod secret_key;

pub use self::c_vec::*;
pub use self::errors::*;
pub use self::hash::*;
pub use self::key::*;
pub use self::message::*;
pub use self::public_key::*;
pub use self::secret_key::*;

/// Free string, that was created by rpgp.
#[no_mangle]
pub unsafe extern "C" fn rpgp_string_drop(p: *mut libc::c_char) {
    let _ = std::ffi::CString::from_raw(p);
    // Drop
}
