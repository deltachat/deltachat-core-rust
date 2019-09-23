use crate::clist::*;
use crate::mailimf_types::*;
use crate::other::*;

/*
  this function creates a new mailimf_fields structure with no fields
*/
pub unsafe fn mailimf_fields_new_empty() -> *mut mailimf_fields {
    let mut list: *mut clist = 0 as *mut clist;
    let mut fields_list: *mut mailimf_fields = 0 as *mut mailimf_fields;
    list = clist_new();
    if list.is_null() {
        return 0 as *mut mailimf_fields;
    }
    fields_list = mailimf_fields_new(list);
    if fields_list.is_null() {
        return 0 as *mut mailimf_fields;
    }
    return fields_list;
}
/*
 this function adds a field to the mailimf_fields structure

 @return MAILIMF_NO_ERROR will be returned on success,
 other code will be returned otherwise
*/
pub unsafe fn mailimf_fields_add(
    mut fields: *mut mailimf_fields,
    mut field: *mut mailimf_field,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = clist_insert_after(
        (*fields).fld_list,
        (*(*fields).fld_list).last,
        field as *mut libc::c_void,
    );
    if r < 0i32 {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

/*
  mailimf_field_new_custom creates a new field of type optional

  @param name should be allocated with malloc()
  @param value should be allocated with malloc()
*/
pub unsafe fn mailimf_field_new_custom(
    mut name: *mut libc::c_char,
    mut value: *mut libc::c_char,
) -> *mut mailimf_field {
    let mut opt_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    opt_field = mailimf_optional_field_new(name, value);
    if !opt_field.is_null() {
        field = mailimf_field_new(
            MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int,
            0 as *mut mailimf_return,
            0 as *mut mailimf_orig_date,
            0 as *mut mailimf_from,
            0 as *mut mailimf_sender,
            0 as *mut mailimf_to,
            0 as *mut mailimf_cc,
            0 as *mut mailimf_bcc,
            0 as *mut mailimf_message_id,
            0 as *mut mailimf_orig_date,
            0 as *mut mailimf_from,
            0 as *mut mailimf_sender,
            0 as *mut mailimf_reply_to,
            0 as *mut mailimf_to,
            0 as *mut mailimf_cc,
            0 as *mut mailimf_bcc,
            0 as *mut mailimf_message_id,
            0 as *mut mailimf_in_reply_to,
            0 as *mut mailimf_references,
            0 as *mut mailimf_subject,
            0 as *mut mailimf_comments,
            0 as *mut mailimf_keywords,
            opt_field,
        );
        if field.is_null() {
            mailimf_optional_field_free(opt_field);
        } else {
            return field;
        }
    }
    return 0 as *mut mailimf_field;
}
