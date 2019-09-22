use libc;

use crate::clist::*;
use crate::mailimf_types::*;
use crate::mmapstring::*;
use crate::other::*;

pub const MAILMIME_MECHANISM_TOKEN: libc::c_uint = 6;
pub const MAILMIME_MECHANISM_BASE64: libc::c_uint = 5;
pub const MAILMIME_MECHANISM_QUOTED_PRINTABLE: libc::c_uint = 4;
pub const MAILMIME_MECHANISM_BINARY: libc::c_uint = 3;
pub const MAILMIME_MECHANISM_8BIT: libc::c_uint = 2;
pub const MAILMIME_MECHANISM_7BIT: libc::c_uint = 1;
pub const MAILMIME_MECHANISM_ERROR: libc::c_uint = 0;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_composite_type {
    pub ct_type: libc::c_int,
    pub ct_token: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_content {
    pub ct_type: *mut mailmime_type,
    pub ct_subtype: *mut libc::c_char,
    pub ct_parameters: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_type {
    pub tp_type: libc::c_int,
    pub tp_data: unnamed,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed {
    pub tp_discrete_type: *mut mailmime_discrete_type,
    pub tp_composite_type: *mut mailmime_composite_type,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_discrete_type {
    pub dt_type: libc::c_int,
    pub dt_extension: *mut libc::c_char,
}
pub type unnamed_0 = libc::c_uint;
pub const MAILMIME_FIELD_LOCATION: unnamed_0 = 8;
pub const MAILMIME_FIELD_LANGUAGE: unnamed_0 = 7;
pub const MAILMIME_FIELD_DISPOSITION: unnamed_0 = 6;
pub const MAILMIME_FIELD_VERSION: unnamed_0 = 5;
pub const MAILMIME_FIELD_DESCRIPTION: unnamed_0 = 4;
pub const MAILMIME_FIELD_ID: unnamed_0 = 3;
pub const MAILMIME_FIELD_TRANSFER_ENCODING: unnamed_0 = 2;
pub const MAILMIME_FIELD_TYPE: unnamed_0 = 1;
pub const MAILMIME_FIELD_NONE: unnamed_0 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_1 {
    pub fld_content: *mut mailmime_content,
    pub fld_encoding: *mut mailmime_mechanism,
    pub fld_id: *mut libc::c_char,
    pub fld_description: *mut libc::c_char,
    pub fld_version: uint32_t,
    pub fld_disposition: *mut mailmime_disposition,
    pub fld_language: *mut mailmime_language,
    pub fld_location: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_language {
    pub lg_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition {
    pub dsp_type: *mut mailmime_disposition_type,
    pub dsp_parms: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition_type {
    pub dsp_type: libc::c_int,
    pub dsp_extension: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_mechanism {
    pub enc_type: libc::c_int,
    pub enc_token: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_fields {
    pub fld_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_parameter {
    pub pa_name: *mut libc::c_char,
    pub pa_value: *mut libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition_parm {
    pub pa_type: libc::c_int,
    pub pa_data: unnamed_3,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_3 {
    pub pa_filename: *mut libc::c_char,
    pub pa_creation_date: *mut libc::c_char,
    pub pa_modification_date: *mut libc::c_char,
    pub pa_read_date: *mut libc::c_char,
    pub pa_size: size_t,
    pub pa_parameter: *mut mailmime_parameter,
}
pub const MAILMIME_DISPOSITION_PARM_PARAMETER: unnamed_11 = 5;
pub const MAILMIME_DISPOSITION_PARM_READ_DATE: unnamed_11 = 3;
pub const MAILMIME_DISPOSITION_PARM_MODIFICATION_DATE: unnamed_11 = 2;
pub const MAILMIME_DISPOSITION_PARM_CREATION_DATE: unnamed_11 = 1;
pub const MAILMIME_DISPOSITION_PARM_FILENAME: unnamed_11 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_multipart_body {
    pub bd_list: *mut clist,
}
pub type unnamed_4 = libc::c_uint;
pub const MAILMIME_DATA_FILE: unnamed_4 = 1;
pub const MAILMIME_DATA_TEXT: unnamed_4 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_data {
    pub dt_type: libc::c_int,
    pub dt_encoding: libc::c_int,
    pub dt_encoded: libc::c_int,
    pub dt_data: unnamed_5,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_5 {
    pub dt_text: unnamed_6,
    pub dt_filename: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_6 {
    pub dt_data: *const libc::c_char,
    pub dt_length: size_t,
}
pub type unnamed_7 = libc::c_uint;
pub const MAILMIME_MESSAGE: unnamed_7 = 3;
pub const MAILMIME_MULTIPLE: unnamed_7 = 2;
pub const MAILMIME_SINGLE: unnamed_7 = 1;
pub const MAILMIME_NONE: unnamed_7 = 0;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime {
    pub mm_parent_type: libc::c_int,
    pub mm_parent: *mut mailmime,
    pub mm_multipart_pos: *mut clistiter,
    pub mm_type: libc::c_int,
    pub mm_mime_start: *const libc::c_char,
    pub mm_length: size_t,
    pub mm_mime_fields: *mut mailmime_fields,
    pub mm_content_type: *mut mailmime_content,
    pub mm_body: *mut mailmime_data,
    pub mm_data: unnamed_8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_8 {
    pub mm_single: *mut mailmime_data,
    pub mm_multipart: unnamed_10,
    pub mm_message: unnamed_9,
}
/* message */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_9 {
    pub mm_fields: *mut mailimf_fields,
    pub mm_msg_mime: *mut mailmime,
}
/* multi-part */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_10 {
    pub mm_preamble: *mut mailmime_data,
    pub mm_epilogue: *mut mailmime_data,
    pub mm_mp_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_encoded_word {
    pub wd_charset: *mut libc::c_char,
    pub wd_text: *mut libc::c_char,
}
pub type unnamed_11 = libc::c_uint;
pub const MAILMIME_DISPOSITION_PARM_SIZE: unnamed_11 = 4;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_section {
    pub sec_list: *mut clist,
}

pub unsafe fn mailmime_attribute_free(mut attribute: *mut libc::c_char) {
    mailmime_token_free(attribute);
}

pub unsafe fn mailmime_token_free(mut token: *mut libc::c_char) {
    free(token as *mut libc::c_void);
}
pub unsafe fn mailmime_composite_type_new(
    mut ct_type: libc::c_int,
    mut ct_token: *mut libc::c_char,
) -> *mut mailmime_composite_type {
    let mut ct: *mut mailmime_composite_type = 0 as *mut mailmime_composite_type;
    ct = malloc(::std::mem::size_of::<mailmime_composite_type>() as libc::size_t)
        as *mut mailmime_composite_type;
    if ct.is_null() {
        return 0 as *mut mailmime_composite_type;
    }
    (*ct).ct_type = ct_type;
    (*ct).ct_token = ct_token;
    return ct;
}

pub unsafe fn mailmime_composite_type_free(mut ct: *mut mailmime_composite_type) {
    if !(*ct).ct_token.is_null() {
        mailmime_extension_token_free((*ct).ct_token);
    }
    free(ct as *mut libc::c_void);
}

pub unsafe fn mailmime_extension_token_free(mut extension: *mut libc::c_char) {
    mailmime_token_free(extension);
}

pub unsafe fn mailmime_content_new(
    mut ct_type: *mut mailmime_type,
    mut ct_subtype: *mut libc::c_char,
    mut ct_parameters: *mut clist,
) -> *mut mailmime_content {
    let mut content: *mut mailmime_content = 0 as *mut mailmime_content;
    content =
        malloc(::std::mem::size_of::<mailmime_content>() as libc::size_t) as *mut mailmime_content;
    if content.is_null() {
        return 0 as *mut mailmime_content;
    }
    (*content).ct_type = ct_type;
    (*content).ct_subtype = ct_subtype;
    (*content).ct_parameters = ct_parameters;
    return content;
}

pub unsafe fn mailmime_content_free(mut content: *mut mailmime_content) {
    mailmime_type_free((*content).ct_type);
    mailmime_subtype_free((*content).ct_subtype);
    if !(*content).ct_parameters.is_null() {
        clist_foreach(
            (*content).ct_parameters,
            ::std::mem::transmute::<Option<unsafe fn(_: *mut mailmime_parameter) -> ()>, clist_func>(
                Some(mailmime_parameter_free),
            ),
            0 as *mut libc::c_void,
        );
        clist_free((*content).ct_parameters);
    }
    free(content as *mut libc::c_void);
}

pub unsafe fn mailmime_parameter_free(mut parameter: *mut mailmime_parameter) {
    mailmime_attribute_free((*parameter).pa_name);
    mailmime_value_free((*parameter).pa_value);
    free(parameter as *mut libc::c_void);
}

pub unsafe fn mailmime_value_free(mut value: *mut libc::c_char) {
    free(value as *mut libc::c_void);
}

pub unsafe fn mailmime_subtype_free(mut subtype: *mut libc::c_char) {
    mailmime_extension_token_free(subtype);
}

pub unsafe fn mailmime_type_free(mut type_0: *mut mailmime_type) {
    match (*type_0).tp_type {
        1 => {
            mailmime_discrete_type_free((*type_0).tp_data.tp_discrete_type);
        }
        2 => {
            mailmime_composite_type_free((*type_0).tp_data.tp_composite_type);
        }
        _ => {}
    }
    free(type_0 as *mut libc::c_void);
}

pub unsafe fn mailmime_discrete_type_free(mut discrete_type: *mut mailmime_discrete_type) {
    if !(*discrete_type).dt_extension.is_null() {
        mailmime_extension_token_free((*discrete_type).dt_extension);
    }
    free(discrete_type as *mut libc::c_void);
}

pub unsafe fn mailmime_description_free(mut description: *mut libc::c_char) {
    free(description as *mut libc::c_void);
}

pub unsafe fn mailmime_location_free(mut location: *mut libc::c_char) {
    free(location as *mut libc::c_void);
}

pub unsafe fn mailmime_discrete_type_new(
    mut dt_type: libc::c_int,
    mut dt_extension: *mut libc::c_char,
) -> *mut mailmime_discrete_type {
    let mut discrete_type: *mut mailmime_discrete_type = 0 as *mut mailmime_discrete_type;
    discrete_type = malloc(::std::mem::size_of::<mailmime_discrete_type>() as libc::size_t)
        as *mut mailmime_discrete_type;
    if discrete_type.is_null() {
        return 0 as *mut mailmime_discrete_type;
    }
    (*discrete_type).dt_type = dt_type;
    (*discrete_type).dt_extension = dt_extension;
    return discrete_type;
}

pub unsafe fn mailmime_encoding_free(mut encoding: *mut mailmime_mechanism) {
    mailmime_mechanism_free(encoding);
}

pub unsafe fn mailmime_mechanism_free(mut mechanism: *mut mailmime_mechanism) {
    if !(*mechanism).enc_token.is_null() {
        mailmime_token_free((*mechanism).enc_token);
    }
    free(mechanism as *mut libc::c_void);
}

pub unsafe fn mailmime_id_free(mut id: *mut libc::c_char) {
    mailimf_msg_id_free(id);
}

pub unsafe fn mailmime_mechanism_new(
    mut enc_type: libc::c_int,
    mut enc_token: *mut libc::c_char,
) -> *mut mailmime_mechanism {
    let mut mechanism: *mut mailmime_mechanism = 0 as *mut mailmime_mechanism;
    mechanism = malloc(::std::mem::size_of::<mailmime_mechanism>() as libc::size_t)
        as *mut mailmime_mechanism;
    if mechanism.is_null() {
        return 0 as *mut mailmime_mechanism;
    }
    (*mechanism).enc_type = enc_type;
    (*mechanism).enc_token = enc_token;
    return mechanism;
}

pub unsafe fn mailmime_parameter_new(
    mut pa_name: *mut libc::c_char,
    mut pa_value: *mut libc::c_char,
) -> *mut mailmime_parameter {
    let mut parameter: *mut mailmime_parameter = 0 as *mut mailmime_parameter;
    parameter = malloc(::std::mem::size_of::<mailmime_parameter>() as libc::size_t)
        as *mut mailmime_parameter;
    if parameter.is_null() {
        return 0 as *mut mailmime_parameter;
    }
    (*parameter).pa_name = pa_name;
    (*parameter).pa_value = pa_value;
    return parameter;
}

pub unsafe fn mailmime_type_new(
    mut tp_type: libc::c_int,
    mut tp_discrete_type: *mut mailmime_discrete_type,
    mut tp_composite_type: *mut mailmime_composite_type,
) -> *mut mailmime_type {
    let mut mime_type: *mut mailmime_type = 0 as *mut mailmime_type;
    mime_type =
        malloc(::std::mem::size_of::<mailmime_type>() as libc::size_t) as *mut mailmime_type;
    if mime_type.is_null() {
        return 0 as *mut mailmime_type;
    }
    (*mime_type).tp_type = tp_type;
    match tp_type {
        1 => (*mime_type).tp_data.tp_discrete_type = tp_discrete_type,
        2 => (*mime_type).tp_data.tp_composite_type = tp_composite_type,
        _ => {}
    }
    return mime_type;
}

pub unsafe fn mailmime_language_new(mut lg_list: *mut clist) -> *mut mailmime_language {
    let mut lang: *mut mailmime_language = 0 as *mut mailmime_language;
    lang = malloc(::std::mem::size_of::<mailmime_language>() as libc::size_t)
        as *mut mailmime_language;
    if lang.is_null() {
        return 0 as *mut mailmime_language;
    }
    (*lang).lg_list = lg_list;
    return lang;
}

pub unsafe fn mailmime_language_free(mut lang: *mut mailmime_language) {
    clist_foreach(
        (*lang).lg_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut libc::c_char) -> ()>, clist_func>(Some(
            mailimf_atom_free,
        )),
        0 as *mut libc::c_void,
    );
    clist_free((*lang).lg_list);
    free(lang as *mut libc::c_void);
}
/*
void mailmime_x_token_free(gchar * x_token);
*/
pub unsafe fn mailmime_field_new(
    mut fld_type: libc::c_int,
    mut fld_content: *mut mailmime_content,
    mut fld_encoding: *mut mailmime_mechanism,
    mut fld_id: *mut libc::c_char,
    mut fld_description: *mut libc::c_char,
    mut fld_version: uint32_t,
    mut fld_disposition: *mut mailmime_disposition,
    mut fld_language: *mut mailmime_language,
    mut fld_location: *mut libc::c_char,
) -> *mut mailmime_field {
    let mut field: *mut mailmime_field = 0 as *mut mailmime_field;
    field = malloc(::std::mem::size_of::<mailmime_field>() as libc::size_t) as *mut mailmime_field;
    if field.is_null() {
        return 0 as *mut mailmime_field;
    }
    (*field).fld_type = fld_type;
    match fld_type {
        1 => (*field).fld_data.fld_content = fld_content,
        2 => (*field).fld_data.fld_encoding = fld_encoding,
        3 => (*field).fld_data.fld_id = fld_id,
        4 => (*field).fld_data.fld_description = fld_description,
        5 => (*field).fld_data.fld_version = fld_version,
        6 => (*field).fld_data.fld_disposition = fld_disposition,
        7 => (*field).fld_data.fld_language = fld_language,
        8 => (*field).fld_data.fld_location = fld_location,
        _ => {}
    }
    return field;
}

pub unsafe fn mailmime_field_free(mut field: *mut mailmime_field) {
    match (*field).fld_type {
        1 => {
            if !(*field).fld_data.fld_content.is_null() {
                mailmime_content_free((*field).fld_data.fld_content);
            }
        }
        2 => {
            if !(*field).fld_data.fld_encoding.is_null() {
                mailmime_encoding_free((*field).fld_data.fld_encoding);
            }
        }
        3 => {
            if !(*field).fld_data.fld_id.is_null() {
                mailmime_id_free((*field).fld_data.fld_id);
            }
        }
        4 => {
            if !(*field).fld_data.fld_description.is_null() {
                mailmime_description_free((*field).fld_data.fld_description);
            }
        }
        6 => {
            if !(*field).fld_data.fld_disposition.is_null() {
                mailmime_disposition_free((*field).fld_data.fld_disposition);
            }
        }
        7 => {
            if !(*field).fld_data.fld_language.is_null() {
                mailmime_language_free((*field).fld_data.fld_language);
            }
        }
        8 => {
            if !(*field).fld_data.fld_location.is_null() {
                mailmime_location_free((*field).fld_data.fld_location);
            }
        }
        _ => {}
    }
    free(field as *mut libc::c_void);
}

pub unsafe fn mailmime_disposition_free(mut dsp: *mut mailmime_disposition) {
    mailmime_disposition_type_free((*dsp).dsp_type);
    clist_foreach(
        (*dsp).dsp_parms,
        ::std::mem::transmute::<
            Option<unsafe fn(_: *mut mailmime_disposition_parm) -> ()>,
            clist_func,
        >(Some(mailmime_disposition_parm_free)),
        0 as *mut libc::c_void,
    );
    clist_free((*dsp).dsp_parms);
    free(dsp as *mut libc::c_void);
}

pub unsafe fn mailmime_disposition_parm_free(mut dsp_parm: *mut mailmime_disposition_parm) {
    match (*dsp_parm).pa_type {
        0 => {
            mailmime_filename_parm_free((*dsp_parm).pa_data.pa_filename);
        }
        1 => {
            mailmime_creation_date_parm_free((*dsp_parm).pa_data.pa_creation_date);
        }
        2 => {
            mailmime_modification_date_parm_free((*dsp_parm).pa_data.pa_modification_date);
        }
        3 => {
            mailmime_read_date_parm_free((*dsp_parm).pa_data.pa_read_date);
        }
        5 => {
            mailmime_parameter_free((*dsp_parm).pa_data.pa_parameter);
        }
        _ => {}
    }
    free(dsp_parm as *mut libc::c_void);
}

pub unsafe fn mailmime_read_date_parm_free(mut date: *mut libc::c_char) {
    mailmime_quoted_date_time_free(date);
}

pub unsafe fn mailmime_quoted_date_time_free(mut date: *mut libc::c_char) {
    mailimf_quoted_string_free(date);
}

pub unsafe fn mailmime_modification_date_parm_free(mut date: *mut libc::c_char) {
    mailmime_quoted_date_time_free(date);
}

pub unsafe fn mailmime_creation_date_parm_free(mut date: *mut libc::c_char) {
    mailmime_quoted_date_time_free(date);
}

pub unsafe fn mailmime_filename_parm_free(mut filename: *mut libc::c_char) {
    mailmime_value_free(filename);
}

pub unsafe fn mailmime_disposition_type_free(mut dsp_type: *mut mailmime_disposition_type) {
    if !(*dsp_type).dsp_extension.is_null() {
        free((*dsp_type).dsp_extension as *mut libc::c_void);
    }
    free(dsp_type as *mut libc::c_void);
}

pub unsafe fn mailmime_fields_new(mut fld_list: *mut clist) -> *mut mailmime_fields {
    let mut fields: *mut mailmime_fields = 0 as *mut mailmime_fields;
    fields =
        malloc(::std::mem::size_of::<mailmime_fields>() as libc::size_t) as *mut mailmime_fields;
    if fields.is_null() {
        return 0 as *mut mailmime_fields;
    }
    (*fields).fld_list = fld_list;
    return fields;
}

pub unsafe fn mailmime_fields_free(mut fields: *mut mailmime_fields) {
    clist_foreach(
        (*fields).fld_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut mailmime_field) -> ()>, clist_func>(Some(
            mailmime_field_free,
        )),
        0 as *mut libc::c_void,
    );
    clist_free((*fields).fld_list);
    free(fields as *mut libc::c_void);
}

pub unsafe fn mailmime_multipart_body_new(mut bd_list: *mut clist) -> *mut mailmime_multipart_body {
    let mut mp_body: *mut mailmime_multipart_body = 0 as *mut mailmime_multipart_body;
    mp_body = malloc(::std::mem::size_of::<mailmime_multipart_body>() as libc::size_t)
        as *mut mailmime_multipart_body;
    if mp_body.is_null() {
        return 0 as *mut mailmime_multipart_body;
    }
    (*mp_body).bd_list = bd_list;
    return mp_body;
}

pub unsafe fn mailmime_multipart_body_free(mut mp_body: *mut mailmime_multipart_body) {
    clist_foreach(
        (*mp_body).bd_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut mailimf_body) -> ()>, clist_func>(Some(
            mailimf_body_free,
        )),
        0 as *mut libc::c_void,
    );
    clist_free((*mp_body).bd_list);
    free(mp_body as *mut libc::c_void);
}

pub unsafe fn mailmime_data_new(
    mut dt_type: libc::c_int,
    mut dt_encoding: libc::c_int,
    mut dt_encoded: libc::c_int,
    mut dt_data: *const libc::c_char,
    mut dt_length: size_t,
    mut dt_filename: *mut libc::c_char,
) -> *mut mailmime_data {
    let mut mime_data: *mut mailmime_data = 0 as *mut mailmime_data;
    mime_data =
        malloc(::std::mem::size_of::<mailmime_data>() as libc::size_t) as *mut mailmime_data;
    if mime_data.is_null() {
        return 0 as *mut mailmime_data;
    }
    (*mime_data).dt_type = dt_type;
    (*mime_data).dt_encoding = dt_encoding;
    (*mime_data).dt_encoded = dt_encoded;
    match dt_type {
        0 => {
            (*mime_data).dt_data.dt_text.dt_data = dt_data;
            (*mime_data).dt_data.dt_text.dt_length = dt_length
        }
        1 => (*mime_data).dt_data.dt_filename = dt_filename,
        _ => {}
    }
    return mime_data;
}

pub unsafe fn mailmime_data_free(mut mime_data: *mut mailmime_data) {
    match (*mime_data).dt_type {
        1 => {
            free((*mime_data).dt_data.dt_filename as *mut libc::c_void);
        }
        _ => {}
    }
    free(mime_data as *mut libc::c_void);
}

pub unsafe fn mailmime_new(
    mut mm_type: libc::c_int,
    mut mm_mime_start: *const libc::c_char,
    mut mm_length: size_t,
    mut mm_mime_fields: *mut mailmime_fields,
    mut mm_content_type: *mut mailmime_content,
    mut mm_body: *mut mailmime_data,
    mut mm_preamble: *mut mailmime_data,
    mut mm_epilogue: *mut mailmime_data,
    mut mm_mp_list: *mut clist,
    mut mm_fields: *mut mailimf_fields,
    mut mm_msg_mime: *mut mailmime,
) -> *mut mailmime {
    let mut mime: *mut mailmime = 0 as *mut mailmime;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    mime = malloc(::std::mem::size_of::<mailmime>() as libc::size_t) as *mut mailmime;
    if mime.is_null() {
        return 0 as *mut mailmime;
    }
    (*mime).mm_parent = 0 as *mut mailmime;
    (*mime).mm_parent_type = MAILMIME_NONE as libc::c_int;
    (*mime).mm_multipart_pos = 0 as *mut clistiter;
    (*mime).mm_type = mm_type;
    (*mime).mm_mime_start = mm_mime_start;
    (*mime).mm_length = mm_length;
    (*mime).mm_mime_fields = mm_mime_fields;
    (*mime).mm_content_type = mm_content_type;
    (*mime).mm_body = mm_body;
    match mm_type {
        1 => (*mime).mm_data.mm_single = mm_body,
        2 => {
            (*mime).mm_data.mm_multipart.mm_preamble = mm_preamble;
            (*mime).mm_data.mm_multipart.mm_epilogue = mm_epilogue;
            (*mime).mm_data.mm_multipart.mm_mp_list = mm_mp_list;
            cur = (*mm_mp_list).first;
            while !cur.is_null() {
                let mut submime: *mut mailmime = 0 as *mut mailmime;
                submime = (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime;
                (*submime).mm_parent = mime;
                (*submime).mm_parent_type = MAILMIME_MULTIPLE as libc::c_int;
                (*submime).mm_multipart_pos = cur;
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell
                }
            }
        }
        3 => {
            (*mime).mm_data.mm_message.mm_fields = mm_fields;
            (*mime).mm_data.mm_message.mm_msg_mime = mm_msg_mime;
            if !mm_msg_mime.is_null() {
                (*mm_msg_mime).mm_parent = mime;
                (*mm_msg_mime).mm_parent_type = MAILMIME_MESSAGE as libc::c_int
            }
        }
        _ => {}
    }
    return mime;
}

pub unsafe fn mailmime_free(mut mime: *mut mailmime) {
    match (*mime).mm_type {
        1 => {
            if (*mime).mm_body.is_null() && !(*mime).mm_data.mm_single.is_null() {
                mailmime_data_free((*mime).mm_data.mm_single);
            }
        }
        2 => {
            /* do nothing */
            if !(*mime).mm_data.mm_multipart.mm_preamble.is_null() {
                mailmime_data_free((*mime).mm_data.mm_multipart.mm_preamble);
            }
            if !(*mime).mm_data.mm_multipart.mm_epilogue.is_null() {
                mailmime_data_free((*mime).mm_data.mm_multipart.mm_epilogue);
            }
            clist_foreach(
                (*mime).mm_data.mm_multipart.mm_mp_list,
                ::std::mem::transmute::<Option<unsafe fn(_: *mut mailmime) -> ()>, clist_func>(
                    Some(mailmime_free),
                ),
                0 as *mut libc::c_void,
            );
            clist_free((*mime).mm_data.mm_multipart.mm_mp_list);
        }
        3 => {
            if !(*mime).mm_data.mm_message.mm_fields.is_null() {
                mailimf_fields_free((*mime).mm_data.mm_message.mm_fields);
            }
            if !(*mime).mm_data.mm_message.mm_msg_mime.is_null() {
                mailmime_free((*mime).mm_data.mm_message.mm_msg_mime);
            }
        }
        _ => {}
    }
    if !(*mime).mm_body.is_null() {
        mailmime_data_free((*mime).mm_body);
    }
    if !(*mime).mm_mime_fields.is_null() {
        mailmime_fields_free((*mime).mm_mime_fields);
    }
    if !(*mime).mm_content_type.is_null() {
        mailmime_content_free((*mime).mm_content_type);
    }
    free(mime as *mut libc::c_void);
}

pub unsafe fn mailmime_encoded_word_new(
    mut wd_charset: *mut libc::c_char,
    mut wd_text: *mut libc::c_char,
) -> *mut mailmime_encoded_word {
    let mut ew: *mut mailmime_encoded_word = 0 as *mut mailmime_encoded_word;
    ew = malloc(::std::mem::size_of::<mailmime_encoded_word>() as libc::size_t)
        as *mut mailmime_encoded_word;
    if ew.is_null() {
        return 0 as *mut mailmime_encoded_word;
    }
    (*ew).wd_charset = wd_charset;
    (*ew).wd_text = wd_text;
    return ew;
}

pub unsafe fn mailmime_encoded_word_free(mut ew: *mut mailmime_encoded_word) {
    mailmime_charset_free((*ew).wd_charset);
    mailmime_encoded_text_free((*ew).wd_text);
    free(ew as *mut libc::c_void);
}

pub unsafe fn mailmime_encoded_text_free(mut text: *mut libc::c_char) {
    free(text as *mut libc::c_void);
}

pub unsafe fn mailmime_charset_free(mut charset: *mut libc::c_char) {
    free(charset as *mut libc::c_void);
}

pub unsafe fn mailmime_disposition_new(
    mut dsp_type: *mut mailmime_disposition_type,
    mut dsp_parms: *mut clist,
) -> *mut mailmime_disposition {
    let mut dsp: *mut mailmime_disposition = 0 as *mut mailmime_disposition;
    dsp = malloc(::std::mem::size_of::<mailmime_disposition>() as libc::size_t)
        as *mut mailmime_disposition;
    if dsp.is_null() {
        return 0 as *mut mailmime_disposition;
    }
    (*dsp).dsp_type = dsp_type;
    (*dsp).dsp_parms = dsp_parms;
    return dsp;
}

pub unsafe fn mailmime_disposition_type_new(
    mut dsp_type: libc::c_int,
    mut dsp_extension: *mut libc::c_char,
) -> *mut mailmime_disposition_type {
    let mut m_dsp_type: *mut mailmime_disposition_type = 0 as *mut mailmime_disposition_type;
    m_dsp_type = malloc(::std::mem::size_of::<mailmime_disposition_type>() as libc::size_t)
        as *mut mailmime_disposition_type;
    if m_dsp_type.is_null() {
        return 0 as *mut mailmime_disposition_type;
    }
    (*m_dsp_type).dsp_type = dsp_type;
    (*m_dsp_type).dsp_extension = dsp_extension;
    return m_dsp_type;
}

pub unsafe fn mailmime_disposition_parm_new(
    mut pa_type: libc::c_int,
    mut pa_filename: *mut libc::c_char,
    mut pa_creation_date: *mut libc::c_char,
    mut pa_modification_date: *mut libc::c_char,
    mut pa_read_date: *mut libc::c_char,
    mut pa_size: size_t,
    mut pa_parameter: *mut mailmime_parameter,
) -> *mut mailmime_disposition_parm {
    let mut dsp_parm: *mut mailmime_disposition_parm = 0 as *mut mailmime_disposition_parm;
    dsp_parm = malloc(::std::mem::size_of::<mailmime_disposition_parm>() as libc::size_t)
        as *mut mailmime_disposition_parm;
    if dsp_parm.is_null() {
        return 0 as *mut mailmime_disposition_parm;
    }
    (*dsp_parm).pa_type = pa_type;
    match pa_type {
        0 => (*dsp_parm).pa_data.pa_filename = pa_filename,
        1 => (*dsp_parm).pa_data.pa_creation_date = pa_creation_date,
        2 => (*dsp_parm).pa_data.pa_modification_date = pa_modification_date,
        3 => (*dsp_parm).pa_data.pa_read_date = pa_read_date,
        4 => (*dsp_parm).pa_data.pa_size = pa_size,
        5 => (*dsp_parm).pa_data.pa_parameter = pa_parameter,
        _ => {}
    }
    return dsp_parm;
}

pub unsafe fn mailmime_section_new(mut sec_list: *mut clist) -> *mut mailmime_section {
    let mut section: *mut mailmime_section = 0 as *mut mailmime_section;
    section =
        malloc(::std::mem::size_of::<mailmime_section>() as libc::size_t) as *mut mailmime_section;
    if section.is_null() {
        return 0 as *mut mailmime_section;
    }
    (*section).sec_list = sec_list;
    return section;
}

pub unsafe fn mailmime_section_free(mut section: *mut mailmime_section) {
    clist_foreach(
        (*section).sec_list,
        ::std::mem::transmute::<Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>, clist_func>(
            Some(free),
        ),
        0 as *mut libc::c_void,
    );
    clist_free((*section).sec_list);
    free(section as *mut libc::c_void);
}

pub unsafe fn mailmime_decoded_part_free(mut part: *mut libc::c_char) {
    mmap_string_unref(part);
}
