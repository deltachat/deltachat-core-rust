use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::ptr;

use charset::Charset;
use deltachat_derive::{FromSql, ToSql};
use mmime::clist::*;
use mmime::mailimf::*;
use mmime::mailimf_types::*;
use mmime::mailmime::*;
use mmime::mailmime_content::*;
use mmime::mailmime_disposition::*;
use mmime::mailmime_types::*;
use mmime::mmapstring::*;
use mmime::other::*;

use crate::constants::Viewtype;
use crate::contact::*;
use crate::context::Context;
use crate::dc_simplify::*;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::e2ee::*;
use crate::location;
use crate::param::*;
use crate::stock::StockMessage;
use crate::x::*;

#[derive(Debug)]
pub struct MimeParser<'a> {
    pub context: &'a Context,
    pub parts: Vec<Part>,
    pub mimeroot: *mut mailmime,
    pub header: HashMap<String, *mut mailimf_field>,
    pub header_root: *mut mailimf_fields,
    pub header_protected: *mut mailimf_fields,
    pub subject: Option<String>,
    pub is_send_by_messenger: bool,
    pub decrypting_failed: bool,
    pub e2ee_helper: E2eeHelper,
    pub is_forwarded: bool,
    pub reports: Vec<*mut mailmime>,
    pub is_system_message: SystemMessage,
    pub location_kml: Option<location::Kml>,
    pub message_kml: Option<location::Kml>,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql)]
#[repr(i32)]
pub enum SystemMessage {
    Unknown = 0,
    GroupNameChanged = 2,
    GroupImageChanged = 3,
    MemberAddedToGroup = 4,
    MemberRemovedFromGroup = 5,
    AutocryptSetupMessage = 6,
    SecurejoinMessage = 7,
    LocationStreamingEnabled = 8,
    LocationOnly = 9,
}

impl Default for SystemMessage {
    fn default() -> Self {
        SystemMessage::Unknown
    }
}

const DC_MIMETYPE_MP_ALTERNATIVE: i32 = 10;
const DC_MIMETYPE_MP_RELATED: i32 = 20;
const DC_MIMETYPE_MP_MIXED: i32 = 30;
const DC_MIMETYPE_MP_NOT_DECRYPTABLE: i32 = 40;
const DC_MIMETYPE_MP_REPORT: i32 = 45;
const DC_MIMETYPE_MP_SIGNED: i32 = 46;
const DC_MIMETYPE_MP_OTHER: i32 = 50;
const DC_MIMETYPE_TEXT_PLAIN: i32 = 60;
const DC_MIMETYPE_TEXT_HTML: i32 = 70;
const DC_MIMETYPE_IMAGE: i32 = 80;
const DC_MIMETYPE_AUDIO: i32 = 90;
const DC_MIMETYPE_VIDEO: i32 = 100;
const DC_MIMETYPE_FILE: i32 = 110;
const DC_MIMETYPE_AC_SETUP_FILE: i32 = 111;

impl<'a> MimeParser<'a> {
    pub fn new(context: &'a Context) -> Self {
        MimeParser {
            parts: Vec::new(),
            mimeroot: std::ptr::null_mut(),
            header: Default::default(),
            header_root: std::ptr::null_mut(),
            header_protected: std::ptr::null_mut(),
            subject: None,
            is_send_by_messenger: false,
            decrypting_failed: false,
            e2ee_helper: Default::default(),
            is_forwarded: false,
            context,
            reports: Vec::new(),
            is_system_message: SystemMessage::Unknown,
            location_kml: None,
            message_kml: None,
        }
    }

    pub unsafe fn parse(&mut self, body: &[u8]) {
        let mut index = 0;

        let r = mailmime_parse(
            body.as_ptr() as *const libc::c_char,
            body.len(),
            &mut index,
            &mut self.mimeroot,
        );

        if r == MAILIMF_NO_ERROR as libc::c_int && !self.mimeroot.is_null() {
            self.e2ee_helper.decrypt(self.context, self.mimeroot);
            self.parse_mime_recursive(self.mimeroot);

            if let Some(field) = self.lookup_field("Subject") {
                if (*field).fld_type == MAILIMF_FIELD_SUBJECT as libc::c_int {
                    let subj = (*(*field).fld_data.fld_subject).sbj_value;

                    self.subject = as_opt_str(subj).map(dc_decode_header_words_safe);
                }
            }

            if self.lookup_optional_field("Chat-Version").is_some() {
                self.is_send_by_messenger = true
            }

            if self.lookup_field("Autocrypt-Setup-Message").is_some() {
                let has_setup_file = self
                    .parts
                    .iter()
                    .any(|p| p.mimetype == DC_MIMETYPE_AC_SETUP_FILE);

                if has_setup_file {
                    self.is_system_message = SystemMessage::AutocryptSetupMessage;

                    // TODO: replace the following code with this
                    // once drain_filter stabilizes.
                    //
                    // See https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain_filter
                    // and https://github.com/rust-lang/rust/issues/43244
                    //
                    // mimeparser
                    //    .parts
                    //    .drain_filter(|part| part.int_mimetype != 111)
                    //    .for_each(|part| dc_mimepart_unref(part));

                    let mut i = 0;
                    while i != self.parts.len() {
                        if self.parts[i].mimetype != 111 {
                            self.parts.remove(i);
                        } else {
                            i += 1;
                        }
                    }
                }
            } else {
                if let Some(optional_field) = self.lookup_optional_field("Chat-Content") {
                    if optional_field == "location-streaming-enabled" {
                        self.is_system_message = SystemMessage::LocationStreamingEnabled;
                    }
                }
            }
            if self.lookup_field("Chat-Group-Image").is_some() && !self.parts.is_empty() {
                let textpart = &self.parts[0];
                if textpart.typ == Viewtype::Text {
                    if self.parts.len() >= 2 {
                        let imgpart = &mut self.parts[1];
                        if imgpart.typ == Viewtype::Image {
                            imgpart.is_meta = true;
                        }
                    }
                }
            }
            if self.is_send_by_messenger && self.parts.len() == 2 {
                let need_drop = {
                    let textpart = &self.parts[0];
                    let filepart = &self.parts[1];
                    textpart.typ == Viewtype::Text
                        && (filepart.typ == Viewtype::Image
                            || filepart.typ == Viewtype::Gif
                            || filepart.typ == Viewtype::Audio
                            || filepart.typ == Viewtype::Voice
                            || filepart.typ == Viewtype::Video
                            || filepart.typ == Viewtype::File)
                        && !filepart.is_meta
                };

                if need_drop {
                    let mut filepart = self.parts.swap_remove(1);

                    // insert new one
                    filepart.msg = self.parts[0].msg.as_ref().map(|s| s.to_string());

                    // forget the one we use now
                    self.parts[0].msg = None;

                    // swap new with old
                    std::mem::replace(&mut self.parts[0], filepart);
                }
            }
            if let Some(ref subject) = self.subject {
                let mut prepend_subject: libc::c_int = 1i32;
                if !self.decrypting_failed {
                    let colon = subject.find(':');
                    if colon == Some(2)
                        || colon == Some(3)
                        || self.is_send_by_messenger
                        || subject.contains("Chat:")
                    {
                        prepend_subject = 0i32
                    }
                }
                if 0 != prepend_subject {
                    let subj = if let Some(n) = subject.find('[') {
                        &subject[0..n]
                    } else {
                        subject
                    }
                    .trim();

                    if !subj.is_empty() {
                        for part in self.parts.iter_mut() {
                            if part.typ == Viewtype::Text {
                                let new_txt = format!(
                                    "{} – {}",
                                    subj,
                                    part.msg.as_ref().expect("missing msg part")
                                );
                                part.msg = Some(new_txt);
                                break;
                            }
                        }
                    }
                }
            }
            if self.is_forwarded {
                for part in self.parts.iter_mut() {
                    part.param.set_int(Param::Forwarded, 1);
                }
            }
            if self.parts.len() == 1 {
                if self.parts[0].typ == Viewtype::Audio {
                    if self.lookup_optional_field("Chat-Voice-Message").is_some() {
                        let part_mut = &mut self.parts[0];
                        part_mut.typ = Viewtype::Voice;
                    }
                }
                let part = &self.parts[0];
                if part.typ == Viewtype::Audio
                    || part.typ == Viewtype::Voice
                    || part.typ == Viewtype::Video
                {
                    if let Some(field_0) = self.lookup_optional_field("Chat-Duration") {
                        let duration_ms = field_0.parse().unwrap_or_default();
                        if duration_ms > 0 && duration_ms < 24 * 60 * 60 * 1000 {
                            let part_mut = &mut self.parts[0];
                            part_mut.param.set_int(Param::Duration, duration_ms);
                        }
                    }
                }
            }
            if !self.decrypting_failed {
                if let Some(dn_field) =
                    self.lookup_optional_field("Chat-Disposition-Notification-To")
                {
                    if self.get_last_nonmeta().is_some() {
                        let mut mb_list: *mut mailimf_mailbox_list = ptr::null_mut();
                        let mut index_0 = 0;
                        let dn_field_c = CString::new(dn_field).unwrap();

                        if mailimf_mailbox_list_parse(
                            dn_field_c.as_ptr(),
                            strlen(dn_field_c.as_ptr()),
                            &mut index_0,
                            &mut mb_list,
                        ) == MAILIMF_NO_ERROR as libc::c_int
                            && !mb_list.is_null()
                        {
                            if let Some(dn_to_addr) = mailimf_find_first_addr(mb_list) {
                                if let Some(from_field) = self.lookup_field("From") {
                                    if (*from_field).fld_type == MAILIMF_FIELD_FROM as libc::c_int
                                        && !(*from_field).fld_data.fld_from.is_null()
                                    {
                                        let from_addr = mailimf_find_first_addr(
                                            (*(*from_field).fld_data.fld_from).frm_mb_list,
                                        );
                                        if let Some(from_addr) = from_addr {
                                            if from_addr == dn_to_addr {
                                                if let Some(part_4) = self.get_last_nonmeta() {
                                                    part_4.param.set_int(Param::WantsMdn, 1);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            mailimf_mailbox_list_free(mb_list);
                        }
                    }
                }
            }
        }
        /* Cleanup - and try to create at least an empty part if there are no parts yet */
        if self.get_last_nonmeta().is_none() && self.reports.is_empty() {
            let mut part_5 = Part::default();
            part_5.typ = Viewtype::Text;
            part_5.msg = Some("".into());

            if let Some(ref subject) = self.subject {
                if !self.is_send_by_messenger {
                    part_5.msg = Some(subject.to_string())
                }
            }
            self.parts.push(part_5);
        }
    }

    pub fn get_last_nonmeta(&mut self) -> Option<&mut Part> {
        self.parts.iter_mut().rev().find(|part| !part.is_meta)
    }

    /* the following functions can be used only after a call to parse() */

    pub fn lookup_field(&self, field_name: &str) -> Option<*mut mailimf_field> {
        match self.header.get(field_name) {
            Some(v) => {
                if v.is_null() {
                    None
                } else {
                    Some(*v)
                }
            }
            None => None,
        }
    }

    pub fn lookup_optional_field(&self, field_name: &str) -> Option<String> {
        if let Some(field) = self.lookup_field_typ(field_name, MAILIMF_FIELD_OPTIONAL_FIELD) {
            let val = unsafe { (*field).fld_data.fld_optional_field };
            if val.is_null() {
                return None;
            } else {
                return Some(unsafe { to_string_lossy((*val).fld_value) });
            }
        }

        None
    }

    pub fn lookup_field_typ(&self, name: &str, typ: u32) -> Option<*const mailimf_field> {
        if let Some(field) = self.lookup_field(name) {
            if unsafe { (*field).fld_type } == typ as libc::c_int {
                Some(field)
            } else {
                None
            }
        } else {
            None
        }
    }

    unsafe fn parse_mime_recursive(&mut self, mime: *mut mailmime) -> bool {
        if mime.is_null() {
            return false;
        }

        if !mailmime_find_ct_parameter(mime, "protected-headers").is_null() {
            let mime = *mime;

            if mime.mm_type == MAILMIME_SINGLE as libc::c_int
                && (*(*mime.mm_content_type).ct_type).tp_type
                    == MAILMIME_TYPE_DISCRETE_TYPE as libc::c_int
                && (*(*(*mime.mm_content_type).ct_type).tp_data.tp_discrete_type).dt_type
                    == MAILMIME_DISCRETE_TYPE_TEXT as libc::c_int
                && !(*mime.mm_content_type).ct_subtype.is_null()
                && &to_string_lossy((*mime.mm_content_type).ct_subtype) == "rfc822-headers"
            {
                info!(
                    self.context,
                    "Protected headers found in text/rfc822-headers attachment: Will be ignored.",
                );
                return false;
            }

            if self.header_protected.is_null() {
                /* use the most outer protected header - this is typically
                created in sync with the normal, unprotected header */
                let mut dummy = 0;
                if mailimf_envelope_and_optional_fields_parse(
                    mime.mm_mime_start,
                    mime.mm_length,
                    &mut dummy,
                    &mut self.header_protected,
                ) != MAILIMF_NO_ERROR as libc::c_int
                    || self.header_protected.is_null()
                {
                    warn!(self.context, "Protected headers parsing error.",);
                } else {
                    hash_header(&mut self.header, self.header_protected);
                }
            } else {
                info!(
                    self.context,
                    "Protected headers found in MIME header: Will be ignored as we already found an outer one."
                );
            }
        }

        match (*mime).mm_type as u32 {
            MAILMIME_SINGLE => self.add_single_part_if_known(mime),
            MAILMIME_MULTIPLE => self.handle_multiple(mime),
            MAILMIME_MESSAGE => {
                if self.header_root.is_null() {
                    self.header_root = (*mime).mm_data.mm_message.mm_fields;
                    hash_header(&mut self.header, self.header_root);
                }
                if (*mime).mm_data.mm_message.mm_msg_mime.is_null() {
                    return false;
                }

                self.parse_mime_recursive((*mime).mm_data.mm_message.mm_msg_mime)
            }
            _ => false,
        }
    }

    unsafe fn handle_multiple(&mut self, mime: *mut mailmime) -> bool {
        let mut any_part_added = false;
        match mailmime_get_mime_type(mime, ptr::null_mut(), ptr::null_mut()) {
            /* Most times, mutlipart/alternative contains true alternatives
            as text/plain and text/html.  If we find a multipart/mixed
            inside mutlipart/alternative, we use this (happens eg in
            apple mail: "plaintext" as an alternative to "html+PDF attachment") */
            DC_MIMETYPE_MP_ALTERNATIVE => {
                for cur_data in (*(*mime).mm_data.mm_multipart.mm_mp_list).into_iter() {
                    if mailmime_get_mime_type(cur_data as *mut _, ptr::null_mut(), ptr::null_mut())
                        == DC_MIMETYPE_MP_MIXED
                    {
                        any_part_added = self.parse_mime_recursive(cur_data as *mut _);
                        break;
                    }
                }
                if !any_part_added {
                    /* search for text/plain and add this */
                    for cur_data in (*(*mime).mm_data.mm_multipart.mm_mp_list).into_iter() {
                        if mailmime_get_mime_type(
                            cur_data as *mut _,
                            ptr::null_mut(),
                            ptr::null_mut(),
                        ) == DC_MIMETYPE_TEXT_PLAIN
                        {
                            any_part_added = self.parse_mime_recursive(cur_data as *mut _);
                            break;
                        }
                    }
                }
                if !any_part_added {
                    /* `text/plain` not found - use the first part */
                    for cur_data in (*(*mime).mm_data.mm_multipart.mm_mp_list).into_iter() {
                        if self.parse_mime_recursive(cur_data as *mut _) {
                            any_part_added = true;
                            break;
                        }
                    }
                }
            }
            DC_MIMETYPE_MP_RELATED => {
                /* add the "root part" - the other parts may be referenced which is
                not interesting for us (eg. embedded images) we assume he "root part"
                being the first one, which may not be always true ...
                however, most times it seems okay. */
                let cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                if !cur.is_null() {
                    any_part_added = self.parse_mime_recursive((*cur).data as *mut mailmime);
                }
            }
            DC_MIMETYPE_MP_NOT_DECRYPTABLE => {
                let mut part = Part::default();
                part.typ = Viewtype::Text;
                let msg_body = self.context.stock_str(StockMessage::CantDecryptMsgBody);

                let txt = format!("[{}]", msg_body);
                part.msg_raw = Some(txt.clone());
                part.msg = Some(txt);

                self.parts.push(part);
                any_part_added = true;
                self.decrypting_failed = true;
            }
            DC_MIMETYPE_MP_SIGNED => {
                /* RFC 1847: "The multipart/signed content type
                contains exactly two body parts.  The first body
                part is the body part over which the digital signature was created [...]
                The second body part contains the control information necessary to
                verify the digital signature." We simpliy take the first body part and
                skip the rest.  (see
                https://k9mail.github.io/2016/11/24/OpenPGP-Considerations-Part-I.html
                for background information why we use encrypted+signed) */
                let cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                if !cur.is_null() {
                    any_part_added = self.parse_mime_recursive((*cur).data as *mut _);
                }
            }
            DC_MIMETYPE_MP_REPORT => {
                /* RFC 6522: the first part is for humans, the second for machines */
                if (*(*mime).mm_data.mm_multipart.mm_mp_list).count >= 2 {
                    let report_type = mailmime_find_ct_parameter(mime, "report-type");
                    if !report_type.is_null()
                        && !(*report_type).pa_value.is_null()
                        && &to_string_lossy((*report_type).pa_value) == "disposition-notification"
                    {
                        self.reports.push(mime);
                    } else {
                        /* eg. `report-type=delivery-status`;
                        maybe we should show them as a little error icon */
                        if !(*(*mime).mm_data.mm_multipart.mm_mp_list).first.is_null() {
                            any_part_added = self.parse_mime_recursive(
                                (*(*(*mime).mm_data.mm_multipart.mm_mp_list).first).data as *mut _,
                            );
                        }
                    }
                }
            }
            _ => {
                /* eg. DC_MIMETYPE_MP_MIXED - add all parts (in fact,
                AddSinglePartIfKnown() later check if the parts are really supported)
                HACK: the following lines are a hack for clients who use
                multipart/mixed instead of multipart/alternative for
                combined text/html messages (eg. Stock Android "Mail" does so).
                So, if we detect such a message below, we skip the HTML
                part.  However, not sure, if there are useful situations to use
                plain+html in multipart/mixed - if so, we should disable the hack. */
                let mut skip_part = ptr::null_mut();
                let mut html_part = ptr::null_mut();
                let mut plain_cnt = 0;
                let mut html_cnt = 0;

                for cur_data in (*(*mime).mm_data.mm_multipart.mm_mp_list).into_iter() {
                    match mailmime_get_mime_type(
                        cur_data as *mut _,
                        ptr::null_mut(),
                        ptr::null_mut(),
                    ) {
                        DC_MIMETYPE_TEXT_PLAIN => {
                            plain_cnt += 1;
                        }
                        DC_MIMETYPE_TEXT_HTML => {
                            html_part = cur_data as *mut mailmime;
                            html_cnt += 1;
                        }
                        _ => {}
                    }
                }
                if plain_cnt == 1 && html_cnt == 1 {
                    warn!(
                        self.context,
                        "HACK: multipart/mixed message found with PLAIN and HTML, we\'ll skip the HTML part as this seems to be unwanted."
                    );
                    skip_part = html_part;
                }

                for cur_data in (*(*mime).mm_data.mm_multipart.mm_mp_list).into_iter() {
                    if cur_data as *mut _ != skip_part {
                        if self.parse_mime_recursive(cur_data as *mut _) {
                            any_part_added = true;
                        }
                    }
                }
            }
        }

        any_part_added
    }

    unsafe fn add_single_part_if_known(&mut self, mime: *mut mailmime) -> bool {
        // return true if a part was added
        if mime.is_null() || (*mime).mm_data.mm_single.is_null() {
            return false;
        }
        let mut raw_mime: *mut libc::c_char = ptr::null_mut();
        let mut msg_type = Viewtype::Unknown;
        let mime_type = mailmime_get_mime_type(mime, &mut msg_type, &mut raw_mime);
        let mime_data = (*mime).mm_data.mm_single;
        if (*mime_data).dt_type != MAILMIME_DATA_TEXT as libc::c_int
            /* MAILMIME_DATA_FILE indicates, the data is in a file; AFAIK this is not used on parsing */
            || (*mime_data).dt_data.dt_text.dt_data.is_null()
            || (*mime_data).dt_data.dt_text.dt_length <= 0
        {
            return false;
        }

        /* mmap_string_unref()'d if set */
        let mut transfer_decoding_buffer: *mut libc::c_char = ptr::null_mut();
        /* must not be free()'d */
        let mut decoded_data: *const libc::c_char = ptr::null();
        let mut decoded_data_bytes = 0;

        if !mailmime_transfer_decode(
            mime,
            &mut decoded_data,
            &mut decoded_data_bytes,
            &mut transfer_decoding_buffer,
        ) {
            /* mailmime_transfer_decode does not allocate when it returns false.
            Note that it's now always an error - might be no data */
            return false;
        }
        let old_part_count = self.parts.len();

        /* regard `Content-Transfer-Encoding:` */
        let mut ok_to_continue = true;
        let mut desired_filename = String::default();
        let mut simplifier: Option<Simplify> = None;
        match mime_type {
            DC_MIMETYPE_TEXT_PLAIN | DC_MIMETYPE_TEXT_HTML => {
                if simplifier.is_none() {
                    simplifier = Some(Simplify::new());
                }
                /* get from `Content-Type: text/...; charset=utf-8`; must not be free()'d */
                let charset = mailmime_content_charset_get((*mime).mm_content_type);
                if !charset.is_null()
                    && strcmp(charset, b"utf-8\x00" as *const u8 as *const libc::c_char) != 0i32
                    && strcmp(charset, b"UTF-8\x00" as *const u8 as *const libc::c_char) != 0i32
                {
                    if let Some(encoding) =
                        Charset::for_label(CStr::from_ptr(charset).to_str().unwrap().as_bytes())
                    {
                        let data = std::slice::from_raw_parts(
                            decoded_data as *const u8,
                            decoded_data_bytes,
                        );

                        let (res, _, _) = encoding.decode(data);
                        if res.is_empty() {
                            /* no error - but nothing to add */
                            ok_to_continue = false;
                        } else {
                            let b = res.as_bytes();
                            decoded_data = b.as_ptr() as *const libc::c_char;
                            decoded_data_bytes = b.len();
                            std::mem::forget(res);
                        }
                    } else {
                        warn!(
                            self.context,
                            "Cannot convert {} bytes from \"{}\" to \"utf-8\".",
                            decoded_data_bytes as libc::c_int,
                            as_str(charset),
                        );
                    }
                }
                if ok_to_continue {
                    /* check header directly as is_send_by_messenger is not yet set up */
                    let is_msgrmsg = self.lookup_optional_field("Chat-Version").is_some();

                    let simplified_txt = if decoded_data_bytes <= 0 || decoded_data.is_null() {
                        "".into()
                    } else {
                        let input_c = strndup(decoded_data, decoded_data_bytes as _);
                        let input = to_string_lossy(input_c);
                        let is_html = mime_type == 70;
                        free(input_c as *mut _);

                        simplifier.unwrap().simplify(&input, is_html, is_msgrmsg)
                    };
                    if !simplified_txt.is_empty() {
                        let mut part = Part::default();
                        part.typ = Viewtype::Text;
                        part.mimetype = mime_type;
                        part.msg = Some(simplified_txt);
                        part.msg_raw = {
                            let raw_c = strndup(decoded_data, decoded_data_bytes as libc::c_ulong);
                            let raw = to_string_lossy(raw_c);
                            free(raw_c.cast());
                            Some(raw)
                        };
                        self.do_add_single_part(part);
                    }

                    if simplifier.unwrap().is_forwarded {
                        self.is_forwarded = true;
                    }
                }
            }
            DC_MIMETYPE_IMAGE
            | DC_MIMETYPE_AUDIO
            | DC_MIMETYPE_VIDEO
            | DC_MIMETYPE_FILE
            | DC_MIMETYPE_AC_SETUP_FILE => {
                /* try to get file name from
                   `Content-Disposition: ... filename*=...`
                or `Content-Disposition: ... filename*0*=... filename*1*=... filename*2*=...`
                or `Content-Disposition: ... filename=...` */
                let mut filename_parts = String::new();

                for cur1 in (*(*(*mime).mm_mime_fields).fld_list).into_iter() {
                    let field = cur1 as *mut mailmime_field;
                    if !field.is_null()
                        && (*field).fld_type == MAILMIME_FIELD_DISPOSITION as libc::c_int
                        && !(*field).fld_data.fld_disposition.is_null()
                    {
                        let file_disposition: *mut mailmime_disposition =
                            (*field).fld_data.fld_disposition;
                        if !file_disposition.is_null() {
                            for cur2 in (*(*file_disposition).dsp_parms).into_iter() {
                                let dsp_param = cur2 as *mut mailmime_disposition_parm;
                                if !dsp_param.is_null() {
                                    if (*dsp_param).pa_type
                                        == MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int
                                        && !(*dsp_param).pa_data.pa_parameter.is_null()
                                        && !(*(*dsp_param).pa_data.pa_parameter).pa_name.is_null()
                                        && strncmp(
                                            (*(*dsp_param).pa_data.pa_parameter).pa_name,
                                            b"filename*\x00" as *const u8 as *const libc::c_char,
                                            9,
                                        ) == 0i32
                                    {
                                        // we assume the filename*?* parts are in order, not seen anything else yet
                                        filename_parts += &to_string_lossy(
                                            (*(*dsp_param).pa_data.pa_parameter).pa_value,
                                        );
                                    } else if (*dsp_param).pa_type
                                        == MAILMIME_DISPOSITION_PARM_FILENAME as libc::c_int
                                    {
                                        // might be a wrongly encoded filename
                                        let s = to_string_lossy((*dsp_param).pa_data.pa_filename);
                                        // this is used only if the parts buffer stays empty
                                        desired_filename = dc_decode_header_words_safe(&s)
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
                if !filename_parts.is_empty() {
                    desired_filename = dc_decode_ext_header(filename_parts.as_bytes()).into_owned();
                }
                if desired_filename.is_empty() {
                    let param = mailmime_find_ct_parameter(mime, "name");
                    if !param.is_null()
                        && !(*param).pa_value.is_null()
                        && 0 != *(*param).pa_value.offset(0isize) as libc::c_int
                    {
                        // might be a wrongly encoded filename
                        desired_filename = to_string_lossy((*param).pa_value);
                    }
                }
                /* if there is still no filename, guess one */
                if desired_filename.is_empty() {
                    if !(*mime).mm_content_type.is_null()
                        && !(*(*mime).mm_content_type).ct_subtype.is_null()
                    {
                        desired_filename =
                            format!("file.{}", as_str((*(*mime).mm_content_type).ct_subtype));
                    } else {
                        ok_to_continue = false;
                    }
                }
                if ok_to_continue {
                    if desired_filename.starts_with("location")
                        && desired_filename.ends_with(".kml")
                    {
                        if !decoded_data.is_null() && decoded_data_bytes > 0 {
                            let d = dc_null_terminate(decoded_data, decoded_data_bytes as i32);
                            self.location_kml = location::Kml::parse(self.context, as_str(d)).ok();
                            free(d.cast());
                        }
                    } else if desired_filename.starts_with("message")
                        && desired_filename.ends_with(".kml")
                    {
                        if !decoded_data.is_null() && decoded_data_bytes > 0 {
                            let d = dc_null_terminate(decoded_data, decoded_data_bytes as i32);
                            self.message_kml = location::Kml::parse(self.context, as_str(d)).ok();
                            free(d.cast());
                        }
                    } else {
                        self.do_add_single_file_part(
                            msg_type,
                            mime_type,
                            as_str(raw_mime),
                            decoded_data,
                            decoded_data_bytes,
                            &desired_filename,
                        );
                    }
                }
            }
            _ => {}
        }
        /* add object? (we do not add all objects, eg. signatures etc. are ignored) */
        if !transfer_decoding_buffer.is_null() {
            mmap_string_unref(transfer_decoding_buffer);
        }
        free(raw_mime as *mut libc::c_void);
        self.parts.len() > old_part_count
    }

    unsafe fn do_add_single_file_part(
        &mut self,
        msg_type: Viewtype,
        mime_type: libc::c_int,
        raw_mime: &str,
        decoded_data: *const libc::c_char,
        decoded_data_bytes: libc::size_t,
        desired_filename: &str,
    ) {
        /* create a free file name to use */
        let path_filename = dc_get_fine_path_filename(self.context, "$BLOBDIR", desired_filename);
        let bytes = std::slice::from_raw_parts(decoded_data as *const u8, decoded_data_bytes);

        /* copy data to file */
        if dc_write_file(self.context, &path_filename, bytes) {
            let mut part = Part::default();
            part.typ = msg_type;
            part.mimetype = mime_type;
            part.bytes = decoded_data_bytes as libc::c_int;
            part.param.set(Param::File, path_filename.to_string_lossy());
            part.param.set(Param::MimeType, raw_mime);
            if mime_type == 80 {
                assert!(!decoded_data.is_null(), "invalid image data");
                let data = std::slice::from_raw_parts(
                    decoded_data as *const u8,
                    decoded_data_bytes as usize,
                );

                if let Ok((width, height)) = dc_get_filemeta(data) {
                    part.param.set_int(Param::Width, width as i32);
                    part.param.set_int(Param::Height, height as i32);
                }
            }
            self.do_add_single_part(part);
        }
    }

    fn do_add_single_part(&mut self, mut part: Part) {
        if self.e2ee_helper.encrypted && self.e2ee_helper.signatures.len() > 0 {
            part.param.set_int(Param::GuranteeE2ee, 1);
        } else if self.e2ee_helper.encrypted {
            part.param.set_int(Param::ErroneousE2ee, 0x2);
        }
        self.parts.push(part);
    }

    pub fn is_mailinglist_message(&self) -> bool {
        if self.lookup_field("List-Id").is_some() {
            return true;
        }

        if let Some(precedence) = self.lookup_optional_field("Precedence") {
            if precedence == "list" || precedence == "bulk" {
                return true;
            }
        }

        false
    }

    pub unsafe fn sender_equals_recipient(&self) -> bool {
        if self.header_root.is_null() {
            return false;
        }

        let mut sender_equals_recipient = false;
        let mut fld_from: *const mailimf_from = ptr::null();

        /* get From: and check there is exactly one sender */
        let fld = mailimf_find_field(self.header_root, MAILIMF_FIELD_FROM as libc::c_int);
        if !(fld.is_null()
            || {
                fld_from = (*fld).fld_data.fld_from;
                fld_from.is_null()
            }
            || (*fld_from).frm_mb_list.is_null()
            || (*(*fld_from).frm_mb_list).mb_list.is_null()
            || (*(*(*fld_from).frm_mb_list).mb_list).count != 1i32)
        {
            let mb = (if !(*(*(*fld_from).frm_mb_list).mb_list).first.is_null() {
                (*(*(*(*fld_from).frm_mb_list).mb_list).first).data
            } else {
                ptr::null_mut()
            }) as *mut mailimf_mailbox;

            if !mb.is_null() {
                let from_addr_norm = addr_normalize(as_str((*mb).mb_addr_spec));
                let recipients = mailimf_get_recipients(self.header_root);
                if recipients.len() == 1 {
                    if recipients.contains(from_addr_norm) {
                        sender_equals_recipient = true;
                    }
                }
            }
        }

        sender_equals_recipient
    }

    pub fn repl_msg_by_error(&mut self, error_msg: impl AsRef<str>) {
        if self.parts.is_empty() {
            return;
        }

        let part = &mut self.parts[0];
        part.typ = Viewtype::Text;
        part.msg = Some(format!("[{}]", error_msg.as_ref()));
        self.parts.truncate(1);

        assert_eq!(self.parts.len(), 1);
    }

    pub fn get_rfc724_mid(&mut self) -> Option<String> {
        // get Message-ID from header
        if let Some(field) = self.lookup_field_typ("Message-ID", MAILIMF_FIELD_MESSAGE_ID) {
            unsafe {
                let fld_message_id = (*field).fld_data.fld_message_id;
                if !fld_message_id.is_null() {
                    return Some(to_string((*fld_message_id).mid_value));
                }
            }
        }
        None
    }
}

impl<'a> Drop for MimeParser<'a> {
    fn drop(&mut self) {
        if !self.header_protected.is_null() {
            unsafe { mailimf_fields_free(self.header_protected) };
        }
        if !self.mimeroot.is_null() {
            unsafe { mailmime_free(self.mimeroot) };
        }
        unsafe { self.e2ee_helper.thanks() };
    }
}

#[derive(Default, Debug, Clone)]
pub struct Part {
    pub typ: Viewtype,
    pub is_meta: bool,
    pub mimetype: i32,
    pub msg: Option<String>,
    pub msg_raw: Option<String>,
    pub bytes: i32,
    pub param: Params,
}

pub fn mailimf_find_first_addr(mb_list: *const mailimf_mailbox_list) -> Option<String> {
    if mb_list.is_null() {
        return None;
    }

    for cur in unsafe { (*(*mb_list).mb_list).into_iter() } {
        let mb = cur as *mut mailimf_mailbox;
        if !mb.is_null() && !unsafe { (*mb).mb_addr_spec.is_null() } {
            let addr = unsafe { as_str((*mb).mb_addr_spec) };
            return Some(addr_normalize(addr).to_string());
        }
    }

    None
}

unsafe fn hash_header(out: &mut HashMap<String, *mut mailimf_field>, in_0: *const mailimf_fields) {
    if in_0.is_null() {
        return;
    }

    for cur in (*(*in_0).fld_list).into_iter() {
        let field = cur as *mut mailimf_field;
        // TODO match on enums /rtn

        let key = match (*field).fld_type as libc::c_uint {
            MAILIMF_FIELD_RETURN_PATH => Some("Return-Path".to_string()),
            MAILIMF_FIELD_ORIG_DATE => Some("Date".to_string()),
            MAILIMF_FIELD_FROM => Some("From".to_string()),
            MAILIMF_FIELD_SENDER => Some("Sender".to_string()),
            MAILIMF_FIELD_REPLY_TO => Some("Reply-To".to_string()),
            MAILIMF_FIELD_TO => Some("To".to_string()),
            MAILIMF_FIELD_CC => Some("Cc".to_string()),
            MAILIMF_FIELD_BCC => Some("Bcc".to_string()),
            MAILIMF_FIELD_MESSAGE_ID => Some("Message-ID".to_string()),
            MAILIMF_FIELD_IN_REPLY_TO => Some("In-Reply-To".to_string()),
            MAILIMF_FIELD_REFERENCES => Some("References".to_string()),
            MAILIMF_FIELD_SUBJECT => Some("Subject".to_string()),
            MAILIMF_FIELD_OPTIONAL_FIELD => {
                // MAILIMF_FIELD_OPTIONAL_FIELD
                let optional_field = (*field).fld_data.fld_optional_field;
                // XXX the optional field sometimes contains invalid UTF8
                // which should not happen (according to the mime standard).
                // This might point to a bug in our mime parsing/processing
                // logic. As mmime/dc_mimeparser is scheduled fore replacement
                // anyway we just use a lossy conversion.

                if !optional_field.is_null() {
                    Some(to_string_lossy((*optional_field).fld_name))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(key) = key {
            if !out.contains_key(&key) || // key already exists, only overwrite known types (protected headers)
                (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as i32 || key.starts_with("Chat-")
            {
                out.insert(key, field);
            }
        }
    }
}

unsafe fn mailmime_get_mime_type(
    mime: *mut mailmime,
    mut msg_type: *mut Viewtype,
    raw_mime: *mut *mut libc::c_char,
) -> libc::c_int {
    let c = (*mime).mm_content_type;
    let mut dummy = Viewtype::Unknown;

    if msg_type.is_null() {
        msg_type = &mut dummy
    }
    *msg_type = Viewtype::Unknown;
    if c.is_null() || (*c).ct_type.is_null() {
        return 0;
    }

    // TODO match on enums /rtn
    match (*(*c).ct_type).tp_type {
        1 => match (*(*(*c).ct_type).tp_data.tp_discrete_type).dt_type {
            1 => {
                if !mailmime_is_attachment_disposition(mime) {
                    if strcmp(
                        (*c).ct_subtype,
                        b"plain\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                    {
                        *msg_type = Viewtype::Text;
                        return DC_MIMETYPE_TEXT_PLAIN;
                    } else {
                        if strcmp(
                            (*c).ct_subtype,
                            b"html\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        {
                            *msg_type = Viewtype::Text;
                            return DC_MIMETYPE_TEXT_HTML;
                        }
                    }
                }
                *msg_type = Viewtype::File;
                if !raw_mime.is_null() {
                    *raw_mime = reconcat_mime(Some("text"), as_opt_str((*c).ct_subtype)).strdup();
                }
                return DC_MIMETYPE_FILE;
            }
            2 => {
                if strcmp(
                    (*c).ct_subtype,
                    b"gif\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    *msg_type = Viewtype::Gif;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"svg+xml\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    *msg_type = Viewtype::File;
                    if !raw_mime.is_null() {
                        *raw_mime =
                            reconcat_mime(Some("image"), as_opt_str((*c).ct_subtype)).strdup();
                    }
                    return DC_MIMETYPE_FILE;
                } else {
                    *msg_type = Viewtype::Image;
                }
                if !raw_mime.is_null() {
                    *raw_mime = reconcat_mime(Some("image"), as_opt_str((*c).ct_subtype)).strdup();
                }
                return DC_MIMETYPE_IMAGE;
            }
            3 => {
                *msg_type = Viewtype::Audio;
                if !raw_mime.is_null() {
                    *raw_mime = reconcat_mime(Some("audio"), as_opt_str((*c).ct_subtype)).strdup();
                }
                return DC_MIMETYPE_AUDIO;
            }
            4 => {
                *msg_type = Viewtype::Video;
                if !raw_mime.is_null() {
                    *raw_mime = reconcat_mime(Some("video"), as_opt_str((*c).ct_subtype)).strdup();
                }
                return DC_MIMETYPE_VIDEO;
            }
            _ => {
                *msg_type = Viewtype::File;
                if (*(*(*c).ct_type).tp_data.tp_discrete_type).dt_type
                    == MAILMIME_DISCRETE_TYPE_APPLICATION as libc::c_int
                    && strcmp(
                        (*c).ct_subtype,
                        b"autocrypt-setup\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                {
                    if !raw_mime.is_null() {
                        *raw_mime = reconcat_mime(None, as_opt_str((*c).ct_subtype)).strdup();
                    }
                    return DC_MIMETYPE_AC_SETUP_FILE;
                }
                if !raw_mime.is_null() {
                    *raw_mime = reconcat_mime(
                        as_opt_str((*(*(*c).ct_type).tp_data.tp_discrete_type).dt_extension),
                        as_opt_str((*c).ct_subtype),
                    )
                    .strdup();
                }
                return DC_MIMETYPE_FILE;
            }
        },
        2 => {
            if (*(*(*c).ct_type).tp_data.tp_composite_type).ct_type
                == MAILMIME_COMPOSITE_TYPE_MULTIPART as libc::c_int
            {
                if strcmp(
                    (*c).ct_subtype,
                    b"alternative\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return DC_MIMETYPE_MP_ALTERNATIVE;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"related\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return DC_MIMETYPE_MP_RELATED;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"encrypted\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return DC_MIMETYPE_MP_NOT_DECRYPTABLE;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"signed\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return DC_MIMETYPE_MP_SIGNED;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"mixed\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return DC_MIMETYPE_MP_MIXED;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"report\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return DC_MIMETYPE_MP_REPORT;
                } else {
                    return DC_MIMETYPE_MP_OTHER;
                }
            } else {
                if (*(*(*c).ct_type).tp_data.tp_composite_type).ct_type
                    == MAILMIME_COMPOSITE_TYPE_MESSAGE as libc::c_int
                {
                    return 0i32;
                }
            }
        }
        _ => {}
    }

    0
}

fn reconcat_mime(typ: Option<&str>, subtype: Option<&str>) -> String {
    let typ = typ.unwrap_or("application");
    let subtype = subtype.unwrap_or("octet-stream");

    format!("{}/{}", typ, subtype)
}

unsafe fn mailmime_is_attachment_disposition(mime: *mut mailmime) -> bool {
    if (*mime).mm_mime_fields.is_null() {
        return false;
    }

    for cur in (*(*(*mime).mm_mime_fields).fld_list).into_iter() {
        let field = cur as *mut mailmime_field;
        if !field.is_null()
            && (*field).fld_type == MAILMIME_FIELD_DISPOSITION as libc::c_int
            && !(*field).fld_data.fld_disposition.is_null()
        {
            if !(*(*field).fld_data.fld_disposition).dsp_type.is_null()
                && (*(*(*field).fld_data.fld_disposition).dsp_type).dsp_type
                    == MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int
            {
                return true;
            }
        }
    }

    false
}

/* low-level-tools for working with mailmime structures directly */
pub unsafe fn mailmime_find_ct_parameter(
    mime: *mut mailmime,
    name: &str,
) -> *mut mailmime_parameter {
    if mime.is_null()
        || (*mime).mm_content_type.is_null()
        || (*(*mime).mm_content_type).ct_parameters.is_null()
    {
        return ptr::null_mut();
    }

    for cur in (*(*(*mime).mm_content_type).ct_parameters).into_iter() {
        let param = cur as *mut mailmime_parameter;
        if !param.is_null() && !(*param).pa_name.is_null() {
            if &to_string_lossy((*param).pa_name) == name {
                return param;
            }
        }
    }

    ptr::null_mut()
}

pub unsafe fn mailmime_transfer_decode(
    mime: *mut mailmime,
    ret_decoded_data: *mut *const libc::c_char,
    ret_decoded_data_bytes: *mut libc::size_t,
    ret_to_mmap_string_unref: *mut *mut libc::c_char,
) -> bool {
    let mut mime_transfer_encoding = MAILMIME_MECHANISM_BINARY as libc::c_int;
    let mime_data: *mut mailmime_data;

    /* must not be free()'d */
    let decoded_data: *const libc::c_char;
    let mut decoded_data_bytes = 0;

    /* mmap_string_unref()'d if set */
    let mut transfer_decoding_buffer: *mut libc::c_char = ptr::null_mut();
    if mime.is_null()
        || ret_decoded_data.is_null()
        || ret_decoded_data_bytes.is_null()
        || ret_to_mmap_string_unref.is_null()
        || !(*ret_decoded_data).is_null()
        || *ret_decoded_data_bytes != 0
        || !(*ret_to_mmap_string_unref).is_null()
    {
        return false;
    }
    mime_data = (*mime).mm_data.mm_single;
    if !(*mime).mm_mime_fields.is_null() {
        for cur in (*(*(*mime).mm_mime_fields).fld_list).into_iter() {
            let field = cur as *mut mailmime_field;

            if !field.is_null()
                && (*field).fld_type == MAILMIME_FIELD_TRANSFER_ENCODING as libc::c_int
                && !(*field).fld_data.fld_encoding.is_null()
            {
                mime_transfer_encoding = (*(*field).fld_data.fld_encoding).enc_type;
                break;
            }
        }
    }

    if mime_transfer_encoding == MAILMIME_MECHANISM_7BIT as libc::c_int
        || mime_transfer_encoding == MAILMIME_MECHANISM_8BIT as libc::c_int
        || mime_transfer_encoding == MAILMIME_MECHANISM_BINARY as libc::c_int
    {
        decoded_data = (*mime_data).dt_data.dt_text.dt_data;
        decoded_data_bytes = (*mime_data).dt_data.dt_text.dt_length;
        if decoded_data.is_null() || decoded_data_bytes <= 0 {
            return false;
        }
    } else {
        let mut current_index = 0;
        let r = mailmime_part_parse(
            (*mime_data).dt_data.dt_text.dt_data,
            (*mime_data).dt_data.dt_text.dt_length,
            &mut current_index,
            mime_transfer_encoding,
            &mut transfer_decoding_buffer,
            &mut decoded_data_bytes,
        );
        if r != MAILIMF_NO_ERROR as libc::c_int
            || transfer_decoding_buffer.is_null()
            || decoded_data_bytes <= 0
        {
            return false;
        }
        decoded_data = transfer_decoding_buffer;
    }

    *ret_decoded_data = decoded_data;
    *ret_decoded_data_bytes = decoded_data_bytes;
    *ret_to_mmap_string_unref = transfer_decoding_buffer;

    true
}

pub unsafe fn mailimf_get_recipients(imffields: *mut mailimf_fields) -> HashSet<String> {
    /* returned addresses are normalized. */
    let mut recipients: HashSet<String> = Default::default();

    for cur in (*(*imffields).fld_list).into_iter() {
        let fld = cur as *mut mailimf_field;

        let fld_to: *mut mailimf_to;
        let fld_cc: *mut mailimf_cc;

        let mut addr_list: *mut mailimf_address_list = ptr::null_mut();
        // TODO match on enums /rtn
        match (*fld).fld_type {
            13 => {
                fld_to = (*fld).fld_data.fld_to;
                if !fld_to.is_null() {
                    addr_list = (*fld_to).to_addr_list
                }
            }
            14 => {
                fld_cc = (*fld).fld_data.fld_cc;
                if !fld_cc.is_null() {
                    addr_list = (*fld_cc).cc_addr_list
                }
            }
            _ => {}
        }

        if !addr_list.is_null() {
            for cur2 in (*(*addr_list).ad_list).into_iter() {
                let adr = cur2 as *mut mailimf_address;

                if !adr.is_null() {
                    if (*adr).ad_type == MAILIMF_ADDRESS_MAILBOX as libc::c_int {
                        mailimf_get_recipients_add_addr(&mut recipients, (*adr).ad_data.ad_mailbox);
                    } else if (*adr).ad_type == MAILIMF_ADDRESS_GROUP as libc::c_int {
                        let group: *mut mailimf_group = (*adr).ad_data.ad_group;
                        if !group.is_null() && !(*group).grp_mb_list.is_null() {
                            for cur3 in (*(*(*group).grp_mb_list).mb_list).into_iter() {
                                mailimf_get_recipients_add_addr(
                                    &mut recipients,
                                    cur3 as *mut mailimf_mailbox,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    recipients
}

fn mailimf_get_recipients_add_addr(recipients: &mut HashSet<String>, mb: *mut mailimf_mailbox) {
    if !mb.is_null() {
        let addr_norm = addr_normalize(as_str(unsafe { (*mb).mb_addr_spec }));
        recipients.insert(addr_norm.into());
    }
}

/*the result is a pointer to mime, must not be freed*/
pub unsafe fn mailimf_find_field(
    header: *mut mailimf_fields,
    wanted_fld_type: libc::c_int,
) -> *mut mailimf_field {
    if header.is_null() || (*header).fld_list.is_null() {
        return ptr::null_mut();
    }
    let mut cur1: *mut clistiter = (*(*header).fld_list).first;
    while !cur1.is_null() {
        let field: *mut mailimf_field = (if !cur1.is_null() {
            (*cur1).data
        } else {
            ptr::null_mut()
        }) as *mut mailimf_field;
        if !field.is_null() {
            if (*field).fld_type == wanted_fld_type {
                return field;
            }
        }
        cur1 = if !cur1.is_null() {
            (*cur1).next
        } else {
            ptr::null_mut()
        }
    }

    ptr::null_mut()
}

/*the result is a pointer to mime, must not be freed*/
pub unsafe fn mailmime_find_mailimf_fields(mime: *mut mailmime) -> *mut mailimf_fields {
    if mime.is_null() {
        return ptr::null_mut();
    }

    match (*mime).mm_type as _ {
        MAILMIME_MULTIPLE => {
            for cur_data in (*(*mime).mm_data.mm_multipart.mm_mp_list).into_iter() {
                let header = mailmime_find_mailimf_fields(cur_data as *mut _);
                if !header.is_null() {
                    return header;
                }
            }
        }
        MAILMIME_MESSAGE => return (*mime).mm_data.mm_message.mm_fields,
        _ => {}
    }

    ptr::null_mut()
}

pub unsafe fn mailimf_find_optional_field(
    header: *mut mailimf_fields,
    wanted_fld_name: *const libc::c_char,
) -> *mut mailimf_optional_field {
    if header.is_null() || (*header).fld_list.is_null() {
        return ptr::null_mut();
    }
    for cur_data in (*(*header).fld_list).into_iter() {
        let field: *mut mailimf_field = cur_data as *mut _;

        if (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            let optional_field: *mut mailimf_optional_field = (*field).fld_data.fld_optional_field;
            if !optional_field.is_null()
                && !(*optional_field).fld_name.is_null()
                && !(*optional_field).fld_value.is_null()
                && strcasecmp((*optional_field).fld_name, wanted_fld_name) == 0i32
            {
                return optional_field;
            }
        }
    }

    ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use proptest::prelude::*;
    use std::ffi::CStr;

    #[test]
    fn test_mailmime_parse() {
        unsafe {
            let txt: *const libc::c_char =
                b"FieldA: ValueA\nFieldB: ValueB\n\x00" as *const u8 as *const libc::c_char;
            let mut mime: *mut mailmime = ptr::null_mut();
            let mut dummy = 0;
            let res = mailmime_parse(txt, strlen(txt), &mut dummy, &mut mime);

            assert_eq!(res, MAIL_NO_ERROR as libc::c_int);
            assert!(!mime.is_null());

            let fields: *mut mailimf_fields = mailmime_find_mailimf_fields(mime);
            assert!(!fields.is_null());

            let mut of_a: *mut mailimf_optional_field = mailimf_find_optional_field(
                fields,
                b"fielda\x00" as *const u8 as *const libc::c_char,
            );

            assert!(!of_a.is_null());
            assert!(!(*of_a).fld_value.is_null());
            assert_eq!(
                CStr::from_ptr((*of_a).fld_name as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "FieldA",
            );
            assert_eq!(
                CStr::from_ptr((*of_a).fld_value as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "ValueA",
            );

            of_a = mailimf_find_optional_field(
                fields,
                b"FIELDA\x00" as *const u8 as *const libc::c_char,
            );

            assert!(!of_a.is_null());
            assert!(!(*of_a).fld_value.is_null());
            assert_eq!(
                CStr::from_ptr((*of_a).fld_name as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "FieldA",
            );
            assert_eq!(
                CStr::from_ptr((*of_a).fld_value as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "ValueA",
            );

            let of_b: *mut mailimf_optional_field = mailimf_find_optional_field(
                fields,
                b"FieldB\x00" as *const u8 as *const libc::c_char,
            );

            assert!(!of_b.is_null());
            assert!(!(*of_b).fld_value.is_null());
            assert_eq!(
                CStr::from_ptr((*of_b).fld_value as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "ValueB",
            );

            mailmime_free(mime);
        }
    }

    #[test]
    fn test_dc_mimeparser_crash() {
        let context = dummy_context();
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mut mimeparser = MimeParser::new(&context.ctx);
        unsafe { mimeparser.parse(&raw[..]) };
        assert_eq!(mimeparser.subject, None);
        assert_eq!(mimeparser.parts.len(), 1);
    }

    proptest! {
        #[test]
        fn test_dc_mailmime_parse_crash_fuzzy(data in "[!-~\t ]{2000,}") {
            let context = dummy_context();
            let mut mimeparser = MimeParser::new(&context.ctx);
            unsafe { mimeparser.parse(data.as_bytes()) };
        }
    }

    #[test]
    fn test_get_rfc724_mid_exists() {
        let context = dummy_context();
        let raw = include_bytes!("../test-data/message/mail_with_message_id.txt");
        let mut mimeparser = MimeParser::new(&context.ctx);
        unsafe { mimeparser.parse(&raw[..]) };
        assert_eq!(
            mimeparser.get_rfc724_mid(),
            Some("2dfdbde7@example.org".into())
        );
    }

    #[test]
    fn test_get_rfc724_mid_not_exists() {
        let context = dummy_context();
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mut mimeparser = MimeParser::new(&context.ctx);
        unsafe { mimeparser.parse(&raw[..]) };
        assert_eq!(mimeparser.get_rfc724_mid(), None);
    }

    #[test]
    fn test_mimeparser_with_context() {
        unsafe {
            let context = dummy_context();
            let raw = b"Content-Type: multipart/mixed; boundary=\"==break==\";\nSubject: outer-subject\nX-Special-A: special-a\nFoo: Bar\nChat-Version: 0.0\n\n--==break==\nContent-Type: text/plain; protected-headers=\"v1\";\nSubject: inner-subject\nX-Special-B: special-b\nFoo: Xy\nChat-Version: 1.0\n\ntest1\n\n--==break==--\n\n\x00";
            let mut mimeparser = MimeParser::new(&context.ctx);
            mimeparser.parse(&raw[..]);

            assert_eq!(mimeparser.subject, Some("inner-subject".into()));

            let of = mimeparser.lookup_optional_field("X-Special-A").unwrap();
            assert_eq!(&of, "special-a");

            let of = mimeparser.lookup_optional_field("Foo").unwrap();
            assert_eq!(&of, "Bar");

            let of = mimeparser.lookup_optional_field("Chat-Version").unwrap();
            assert_eq!(&of, "1.0");
            assert_eq!(mimeparser.parts.len(), 1);
        }
    }
}
