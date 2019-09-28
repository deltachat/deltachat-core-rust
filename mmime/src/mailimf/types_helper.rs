use crate::clist::*;
use crate::mailimf::types::*;
use crate::other::*;

/*
  this function creates a new mailimf_fields structure with no fields
*/
pub unsafe fn mailimf_fields_new_empty() -> *mut mailimf_fields {
    mailimf_fields_new(Vec::new())
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
        field = mailimf_field::OptionalField(opt_field);
        if field.is_null() {
            mailimf_optional_field_free(opt_field);
        } else {
            return field;
        }
    }
    return 0 as *mut mailimf_field;
}
