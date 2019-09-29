use crate::mailimf::types::*;

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
pub unsafe fn mailimf_fields_add(fields: *mut mailimf_fields, field: mailimf_field) {
    (*fields).0.push(field)
}

/*
  mailimf_field_new_custom creates a new field of type optional

  @param name should be allocated with malloc()
  @param value should be allocated with malloc()
*/
pub unsafe fn mailimf_field_new_custom(
    name: *mut libc::c_char,
    value: *mut libc::c_char,
) -> mailimf_field {
    let opt_field = mailimf_optional_field_new(name, value);
    assert!(!opt_field.is_null(), "failed memory allocation");

    mailimf_field::OptionalField(opt_field)
}
