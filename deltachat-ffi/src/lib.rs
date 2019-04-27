#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]
use deltachat::*;
use libc;

pub const DC_VERSION_STR: &'static str = "0.43.0\x00";


// dc_context_t

#[no_mangle]
pub type dc_context_t = dc_context::dc_context_t;


// dc_array_t

#[no_mangle]
pub type dc_array_t = dc_array::dc_array_t;

#[no_mangle]
pub unsafe extern "C" fn dc_array_unref(a: *mut dc_array::dc_array_t) {
    dc_array::dc_array_unref(a)
}

#[no_mangle]
pub unsafe extern "C" fn dc_array_add_uint(array: *mut dc_array_t, item: libc::c_ulong) {
    dc_array::dc_array_add_uint(array, item)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_add_id(array: *mut dc_array_t, item: libc::c_uint) {
    dc_array::dc_array_add_id(array, item)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_add_ptr(array: *mut dc_array_t, item: *mut libc::c_void) {
    dc_array::dc_array_add_ptr(array, item)
}

#[no_mangle]
pub unsafe extern "C" fn dc_array_get_cnt(array: *const dc_array_t) -> libc::c_ulong {
    dc_array::dc_array_get_cnt(array)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_uint(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_ulong {
    dc_array::dc_array_get_uint(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_id(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_uint {
    dc_array::dc_array_get_id(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_ptr(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> *mut libc::c_void {
    dc_array::dc_array_get_ptr(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_latitude(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_double {
    dc_array::dc_array_get_latitude(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_longitude(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_double {
    dc_array::dc_array_get_longitude(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_accuracy(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_double {
    dc_array::dc_array_get_accuracy(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_timestamp(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_long {
    dc_array::dc_array_get_timestamp(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_chat_id(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_uint {
    dc_array::dc_array_get_chat_id(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_contact_id(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_uint {
    dc_array::dc_array_get_contact_id(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_msg_id(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> libc::c_uint {
    dc_array::dc_array_get_msg_id(array, index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_marker(
    array: *const dc_array_t,
    index: libc::c_ulong,
) -> *mut libc::c_char {
    dc_array::dc_array_get_marker(array, index)
}

#[no_mangle]
pub unsafe extern "C" fn dc_array_search_id(
    array: *const dc_array_t,
    needle: libc::c_uint,
    ret_index: *mut libc::c_ulong,
) -> libc::c_int {
    dc_array::dc_array_search_id(array, needle, ret_index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_raw(array: *const dc_array_t) -> *const libc::c_ulong {
    dc_array::dc_array_get_raw(array)
}


// dc_chatlist_t

#[no_mangle]
pub type dc_chatlist_t = dc_chatlist::dc_chatlist_t;

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_unref(chatlist: *mut dc_chatlist::dc_chatlist_t) {
    dc_chatlist::dc_chatlist_unref(chatlist)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_cnt(chatlist: *mut dc_chatlist::dc_chatlist_t) -> libc::c_ulong {
    dc_chatlist::dc_chatlist_get_cnt(chatlist)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_chat_id(chatlist: *mut dc_chatlist::dc_chatlist_t, index: libc::c_ulong) -> libc::uint32_t {
    dc_chatlist::dc_chatlist_get_chat_id(chatlist, index)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_msg_id(chatlist: *mut dc_chatlist::dc_chatlist_t, index: libc::c_ulong) -> libc::uint32_t {
    dc_chatlist::dc_chatlist_get_msg_id(chatlist, index)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_summary(chatlist: *mut dc_chatlist::dc_chatlist_t, index: libc::c_ulong, chat: *mut dc_chat::dc_chat_t) -> *mut dc_lot::dc_lot_t {
    dc_chatlist::dc_chatlist_get_summary(chatlist, index, chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_context(chatlist: *mut dc_chatlist::dc_chatlist_t) -> *mut dc_context::dc_context_t {
    dc_chatlist::dc_chatlist_get_context(chatlist)
}


// dc_chat_t

#[no_mangle]
pub type dc_chat_t = dc_chat::dc_chat_t;

#[no_mangle]
pub unsafe extern "C" fn dc_chat_unref(chat: *mut dc_chat::dc_chat_t) {
    dc_chat::dc_chat_unref(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_id(chat: *mut dc_chat::dc_chat_t) -> libc::uint32_t {
    dc_chat::dc_chat_get_id(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_type(chat: *mut dc_chat::dc_chat_t) -> libc::c_int {
    dc_chat::dc_chat_get_type(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_name(chat: *mut dc_chat::dc_chat_t) -> *mut libc::c_char {
    dc_chat::dc_chat_get_name(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_subtitle(chat: *mut dc_chat::dc_chat_t) -> *mut libc::c_char {
    dc_chat::dc_chat_get_subtitle(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_profile_image(chat: *mut dc_chat::dc_chat_t) -> *mut libc::c_char {
    dc_chat::dc_chat_get_profile_image(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_color(chat: *mut dc_chat::dc_chat_t) -> libc::uint32_t {
    dc_chat::dc_chat_get_color(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_archived(chat: *mut dc_chat::dc_chat_t) -> libc::c_int {
    dc_chat::dc_chat_get_archived(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_unpromoted(chat: *mut dc_chat::dc_chat_t) -> libc::c_int {
    dc_chat::dc_chat_is_unpromoted(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_self_talk(chat: *mut dc_chat::dc_chat_t) -> libc::c_int {
    dc_chat::dc_chat_is_self_talk(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_verified(chat: *mut dc_chat::dc_chat_t)-> libc::c_int {
    dc_chat::dc_chat_is_verified(chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_sending_locations(chat: *mut dc_chat::dc_chat_t) -> libc::c_int {
    dc_chat::dc_chat_is_sending_locations(chat)
}


// dc_msg_t

#[no_mangle]
pub type dc_msg_t = dc_msg::dc_msg_t;

#[no_mangle]
pub unsafe extern "C" fn dc_msg_new(context: *mut dc_context::dc_context_t, viewtype: libc::c_int) -> *mut dc_msg::dc_msg_t {
    dc_msg::dc_msg_new(context, viewtype)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_unref (msg: *mut dc_msg::dc_msg_t) {
    dc_msg::dc_msg_unref(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_empty (msg: *mut dc_msg::dc_msg_t) {
    dc_msg::dc_msg_empty(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_id (msg: *mut dc_msg::dc_msg_t) -> libc::uint32_t {
    dc_msg::dc_msg_get_id(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_from_id (msg: *mut dc_msg::dc_msg_t) -> libc::uint32_t {
    dc_msg::dc_msg_get_from_id(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_chat_id (msg: *mut dc_msg::dc_msg_t) -> libc::uint32_t {
    dc_msg::dc_msg_get_chat_id(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_viewtype (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_get_viewtype(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_state (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_get_state(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_timestamp (msg: *mut dc_msg::dc_msg_t) -> libc::int64_t {
    dc_msg::dc_msg_get_timestamp(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_received_timestamp (msg: *mut dc_msg::dc_msg_t) -> libc::int64_t {
    dc_msg::dc_msg_get_received_timestamp(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_sort_timestamp (msg: *mut dc_msg::dc_msg_t) -> libc::int64_t {
    dc_msg::dc_msg_get_sort_timestamp(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_text (msg: *mut dc_msg::dc_msg_t) -> *mut libc::c_char {
    dc_msg::dc_msg_get_text(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_file (msg: *mut dc_msg::dc_msg_t) -> *mut libc::c_char {
    dc_msg::dc_msg_get_file(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filename (msg: *mut dc_msg::dc_msg_t) -> *mut libc::c_char {
    dc_msg::dc_msg_get_filename(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filemime (msg: *mut dc_msg::dc_msg_t) -> *mut libc::c_char {
    dc_msg::dc_msg_get_filemime(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filebytes (msg: *mut dc_msg::dc_msg_t) -> libc::uint64_t {
    dc_msg::dc_msg_get_filebytes(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_width (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_get_width(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_height (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_get_height(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_duration (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_get_duration(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_showpadlock (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_get_showpadlock(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summary (msg: *mut dc_msg::dc_msg_t, chat: *mut dc_chat::dc_chat_t) -> *mut dc_lot::dc_lot_t {
    dc_msg::dc_msg_get_summary(msg, chat)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summarytext (msg: *mut dc_msg::dc_msg_t, approx_characters: libc::c_int) -> *mut libc::c_char {
    dc_msg::dc_msg_get_summarytext(msg, approx_characters)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_deviating_timestamp(msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_has_deviating_timestamp(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_location (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_has_location(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_sent (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_is_sent(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_starred (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_is_starred(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_forwarded (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_is_forwarded(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_info (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_is_info(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_increation (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_is_increation(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_setupmessage (msg: *mut dc_msg::dc_msg_t) -> libc::c_int {
    dc_msg::dc_msg_is_setupmessage(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_setupcodebegin (msg: *mut dc_msg::dc_msg_t) -> *mut libc::c_char {
    dc_msg::dc_msg_get_setupcodebegin(msg)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_text (msg: *mut dc_msg::dc_msg_t, text: *mut libc::c_char) {
    dc_msg::dc_msg_set_text(msg, text)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_file (msg: *mut dc_msg::dc_msg_t, file: *mut libc::c_char, filemime: *mut libc::c_char) {
    dc_msg::dc_msg_set_file(msg, file, filemime)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_dimension(msg: *mut dc_msg::dc_msg_t, width: libc::c_int, height: libc::c_int) {
    dc_msg::dc_msg_set_dimension(msg, width, height)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_duration (msg: *mut dc_msg::dc_msg_t, duration: libc::c_int) {
    dc_msg::dc_msg_set_duration(msg, duration)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_location (msg: *mut dc_msg::dc_msg_t, latitude: libc::c_double, longitude: libc::c_double) {
    dc_msg::dc_msg_set_location(msg, latitude, longitude)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_latefiling_mediasize (msg: *mut dc_msg::dc_msg_t, width: libc::c_int, height: libc::c_int, duration: libc::c_int) {
    dc_msg::dc_msg_latefiling_mediasize(msg, width, height, duration)
}


// dc_lot_t

#[no_mangle]
pub type dc_lot_t = dc_lot::dc_lot_t;

