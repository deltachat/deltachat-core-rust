extern crate deltachat_provider_database;

use std::ptr;

use deltachat::dc_tools::{as_str, StrExt};
use deltachat_provider_database::StatusState;

#[no_mangle]
pub type dc_provider_t = deltachat_provider_database::Provider;

#[no_mangle]
pub unsafe extern "C" fn dc_provider_new_from_domain(
    domain: *const libc::c_char,
) -> *const dc_provider_t {
    match deltachat_provider_database::get_provider_info(as_str(domain)) {
        Some(provider) => provider,
        None => ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_new_from_email(
    email: *const libc::c_char,
) -> *const dc_provider_t {
    let domain = deltachat_provider_database::get_domain_from_email(as_str(email));
    match deltachat_provider_database::get_provider_info(domain) {
        Some(provider) => provider,
        None => ptr::null(),
    }
}

macro_rules! null_guard {
    ($context:tt) => {
        if $context.is_null() {
            return ptr::null_mut() as *mut libc::c_char;
        }
    };
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_overview_page(
    provider: *const dc_provider_t,
) -> *mut libc::c_char {
    null_guard!(provider);
    format!(
        "{}/{}",
        deltachat_provider_database::PROVIDER_OVERVIEW_URL,
        (*provider).overview_page
    )
    .strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_name(provider: *const dc_provider_t) -> *mut libc::c_char {
    null_guard!(provider);
    (*provider).name.strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_markdown(
    provider: *const dc_provider_t,
) -> *mut libc::c_char {
    null_guard!(provider);
    (*provider).markdown.strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_status_date(
    provider: *const dc_provider_t,
) -> *mut libc::c_char {
    null_guard!(provider);
    (*provider).status.date.strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_status(provider: *const dc_provider_t) -> u32 {
    if provider.is_null() {
        return 0;
    }
    match (*provider).status.state {
        StatusState::OK => 1,
        StatusState::PREPARATION => 2,
        StatusState::BROKEN => 3,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_unref(_provider: *const dc_provider_t) {
    ()
}

// TODO expose general provider overview url?
