use libc;

use crate::constants::Event;
use crate::dc_contact::*;
use crate::dc_context::dc_context_t;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/* Return the string with the given ID by calling DC_EVENT_GET_STRING.
The result must be free()'d! */
pub unsafe fn dc_stock_str(
    mut context: *mut dc_context_t,
    mut id: libc::c_int,
) -> *mut libc::c_char {
    return get_string(context, id, 0i32);
}
unsafe fn get_string(
    mut context: *mut dc_context_t,
    mut id: libc::c_int,
    mut qty: libc::c_int,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !context.is_null() {
        ret = (*context).cb.expect("non-null function pointer")(
            context,
            Event::GET_STRING,
            id as uintptr_t,
            qty as uintptr_t,
        ) as *mut libc::c_char;
    }
    if ret.is_null() {
        ret = default_string(id)
    }
    return ret;
}
/* Add translated strings that are used by the messager backend.
As the logging functions may use these strings, do not log any
errors from here. */
unsafe fn default_string(mut id: libc::c_int) -> *mut libc::c_char {
    match id {
        1 => {
            return dc_strdup(b"No messages.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        2 => {
            return dc_strdup(b"Me\x00" as *const u8 as *const libc::c_char)
        }
        3 => {
            return dc_strdup(b"Draft\x00" as *const u8 as *const libc::c_char)
        }
        4 => {
            return dc_strdup(b"%1$s member(s)\x00" as *const u8 as
                                 *const libc::c_char)
        }
        6 => {
            return dc_strdup(b"%1$s contact(s)\x00" as *const u8 as
                                 *const libc::c_char)
        }
        7 => {
            return dc_strdup(b"Voice message\x00" as *const u8 as
                                 *const libc::c_char)
        }
        8 => {
            return dc_strdup(b"Contact requests\x00" as *const u8 as
                                 *const libc::c_char)
        }
        9 => {
            return dc_strdup(b"Image\x00" as *const u8 as *const libc::c_char)
        }
        23 => {
            return dc_strdup(b"GIF\x00" as *const u8 as *const libc::c_char)
        }
        10 => {
            return dc_strdup(b"Video\x00" as *const u8 as *const libc::c_char)
        }
        11 => {
            return dc_strdup(b"Audio\x00" as *const u8 as *const libc::c_char)
        }
        12 => {
            return dc_strdup(b"File\x00" as *const u8 as *const libc::c_char)
        }
        66 => {
            return dc_strdup(b"Location\x00" as *const u8 as
                                 *const libc::c_char)
        }
        24 => {
            return dc_strdup(b"Encrypted message\x00" as *const u8 as
                                 *const libc::c_char)
        }
        13 => {
            return dc_strdup(b"Sent with my Delta Chat Messenger: https://delta.chat\x00"
                                 as *const u8 as *const libc::c_char)
        }
        14 => {
            return dc_strdup(b"Hello, I\'ve just created the group \"%1$s\" for us.\x00"
                                 as *const u8 as *const libc::c_char)
        }
        15 => {
            return dc_strdup(b"Group name changed from \"%1$s\" to \"%2$s\".\x00"
                                 as *const u8 as *const libc::c_char)
        }
        16 => {
            return dc_strdup(b"Group image changed.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        17 => {
            return dc_strdup(b"Member %1$s added.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        18 => {
            return dc_strdup(b"Member %1$s removed.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        19 => {
            return dc_strdup(b"Group left.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        64 => {
            return dc_strdup(b"Location streaming enabled.\x00" as *const u8
                                 as *const libc::c_char)
        }
        65 => {
            return dc_strdup(b"Location streaming disabled.\x00" as *const u8
                                 as *const libc::c_char)
        }
        62 => {
            return dc_strdup(b"%1$s by %2$s.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        63 => {
            return dc_strdup(b"%1$s by me.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        25 => {
            return dc_strdup(b"End-to-end encryption available.\x00" as
                                 *const u8 as *const libc::c_char)
        }
        27 => {
            return dc_strdup(b"Transport-encryption.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        28 => {
            return dc_strdup(b"No encryption.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        30 => {
            return dc_strdup(b"Fingerprints\x00" as *const u8 as
                                 *const libc::c_char)
        }
        31 => {
            return dc_strdup(b"Return receipt\x00" as *const u8 as
                                 *const libc::c_char)
        }
        32 => {
            return dc_strdup(b"This is a return receipt for the message \"%1$s\".\x00"
                                 as *const u8 as *const libc::c_char)
        }
        33 => {
            return dc_strdup(b"Group image deleted.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        34 => {
            return dc_strdup(b"End-to-end encryption preferred.\x00" as
                                 *const u8 as *const libc::c_char)
        }
        35 => {
            return dc_strdup(b"%1$s verified.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        36 => {
            return dc_strdup(b"Cannot verifiy %1$s\x00" as *const u8 as
                                 *const libc::c_char)
        }
        37 => {
            return dc_strdup(b"Changed setup for %1$s\x00" as *const u8 as
                                 *const libc::c_char)
        }
        40 => {
            return dc_strdup(b"Archived chats\x00" as *const u8 as
                                 *const libc::c_char)
        }
        41 => {
            return dc_strdup(b"Starred messages\x00" as *const u8 as
                                 *const libc::c_char)
        }
        42 => {
            return dc_strdup(b"Autocrypt Setup Message\x00" as *const u8 as
                                 *const libc::c_char)
        }
        43 => {
            return dc_strdup(b"This is the Autocrypt Setup Message used to transfer your key between clients.\n\nTo decrypt and use your key, open the message in an Autocrypt-compliant client and enter the setup code presented on the generating device.\x00"
                                 as *const u8 as *const libc::c_char)
        }
        50 => {
            return dc_strdup(b"Messages I sent to myself\x00" as *const u8 as
                                 *const libc::c_char)
        }
        29 => {
            return dc_strdup(b"This message was encrypted for another setup.\x00"
                                 as *const u8 as *const libc::c_char)
        }
        60 => {
            return dc_strdup(b"Cannot login as %1$s.\x00" as *const u8 as
                                 *const libc::c_char)
        }
        61 => {
            return dc_strdup(b"Response from %1$s: %2$s\x00" as *const u8 as
                                 *const libc::c_char)
        }
        _ => { }
    }
    return dc_strdup(b"ErrStr\x00" as *const u8 as *const libc::c_char);
}
/* Replaces the first `%1$s` in the given String-ID by the given value.
The result must be free()'d! */
pub unsafe fn dc_stock_str_repl_string(
    mut context: *mut dc_context_t,
    mut id: libc::c_int,
    mut to_insert: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = get_string(context, id, 0i32);
    dc_str_replace(
        &mut ret,
        b"%1$s\x00" as *const u8 as *const libc::c_char,
        to_insert,
    );
    dc_str_replace(
        &mut ret,
        b"%1$d\x00" as *const u8 as *const libc::c_char,
        to_insert,
    );
    return ret;
}
pub unsafe fn dc_stock_str_repl_int(
    mut context: *mut dc_context_t,
    mut id: libc::c_int,
    mut to_insert_int: libc::c_int,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = get_string(context, id, to_insert_int);
    let mut to_insert_str: *mut libc::c_char = dc_mprintf(
        b"%i\x00" as *const u8 as *const libc::c_char,
        to_insert_int as libc::c_int,
    );
    dc_str_replace(
        &mut ret,
        b"%1$s\x00" as *const u8 as *const libc::c_char,
        to_insert_str,
    );
    dc_str_replace(
        &mut ret,
        b"%1$d\x00" as *const u8 as *const libc::c_char,
        to_insert_str,
    );
    free(to_insert_str as *mut libc::c_void);
    return ret;
}
/* Replaces the first `%1$s` and `%2$s` in the given String-ID by the two given strings.
The result must be free()'d! */
pub unsafe fn dc_stock_str_repl_string2(
    mut context: *mut dc_context_t,
    mut id: libc::c_int,
    mut to_insert: *const libc::c_char,
    mut to_insert2: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = get_string(context, id, 0i32);
    dc_str_replace(
        &mut ret,
        b"%1$s\x00" as *const u8 as *const libc::c_char,
        to_insert,
    );
    dc_str_replace(
        &mut ret,
        b"%1$d\x00" as *const u8 as *const libc::c_char,
        to_insert,
    );
    dc_str_replace(
        &mut ret,
        b"%2$s\x00" as *const u8 as *const libc::c_char,
        to_insert2,
    );
    dc_str_replace(
        &mut ret,
        b"%2$d\x00" as *const u8 as *const libc::c_char,
        to_insert2,
    );
    return ret;
}
/* Misc. */
pub unsafe fn dc_stock_system_msg(
    mut context: *mut dc_context_t,
    mut str_id: libc::c_int,
    mut param1: *const libc::c_char,
    mut param2: *const libc::c_char,
    mut from_id: uint32_t,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut mod_contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut mod_displayname: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut from_contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut from_displayname: *mut libc::c_char = 0 as *mut libc::c_char;
    if str_id == 17i32 || str_id == 18i32 {
        let mut mod_contact_id: uint32_t = dc_lookup_contact_id_by_addr(context, param1);
        if mod_contact_id != 0i32 as libc::c_uint {
            mod_contact = dc_get_contact(context, mod_contact_id);
            mod_displayname = dc_contact_get_name_n_addr(mod_contact);
            param1 = mod_displayname
        }
    }
    let mut action: *mut libc::c_char = dc_stock_str_repl_string2(context, str_id, param1, param2);
    if 0 != from_id {
        if 0 != strlen(action)
            && *action.offset(strlen(action).wrapping_sub(1) as isize) as libc::c_int == '.' as i32
        {
            *action.offset(strlen(action).wrapping_sub(1) as isize) = 0i32 as libc::c_char
        }
        from_contact = dc_get_contact(context, from_id);
        from_displayname = dc_contact_get_display_name(from_contact);
        ret = dc_stock_str_repl_string2(
            context,
            if from_id == 1i32 as libc::c_uint {
                63i32
            } else {
                62i32
            },
            action,
            from_displayname,
        )
    } else {
        ret = dc_strdup(action)
    }
    free(action as *mut libc::c_void);
    free(from_displayname as *mut libc::c_void);
    free(mod_displayname as *mut libc::c_void);
    dc_contact_unref(from_contact);
    dc_contact_unref(mod_contact);
    return ret;
}
