extern crate deltachat_provider_database;

use crate::string::{to_string_lossy, StrExt};

#[no_mangle]
pub type dc_provider_t = deltachat_provider_database::Provider;

#[no_mangle]
pub unsafe extern "C" fn dc_provider_json_from_domain(
    domain: *const libc::c_char,
) -> *mut libc::c_char {
    let domain = to_string_lossy(domain);
    match deltachat_provider_database::get_provider_info(&domain) {
        Some(provider) => serde_json::to_string(provider).unwrap_or("".to_owned()).strdup(),
        None => "".strdup(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_json_from_email(
    email: *const libc::c_char,
) -> *mut libc::c_char {
    let email = to_string_lossy(email);
    let domain = deltachat_provider_database::get_domain_from_email(&email);
    match deltachat_provider_database::get_provider_info(domain) {
        Some(provider) => serde_json::to_string(provider).unwrap_or("".to_owned()).strdup(),
        None => "".strdup(),
    }
}

// TODO expose general provider overview url?
