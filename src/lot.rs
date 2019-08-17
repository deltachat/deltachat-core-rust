use crate::chat::*;
use crate::constants::Chattype;
use crate::contact::*;
use crate::context::Context;
use crate::dc_msg::*;
use crate::dc_tools::*;
use crate::stock::StockMessage;
use crate::x::*;

/// An object containing a set of values.
/// The meaning of the values is defined by the function returning the object.
/// Lot objects are created
/// eg. by chatlist.get_summary() or dc_msg_get_summary().
///
/// _Lot_ is used in the meaning _heap_ here.
#[derive(Clone)]
pub struct Lot {
    pub(crate) text1_meaning: i32,
    pub(crate) text1: *mut libc::c_char,
    pub(crate) text2: *mut libc::c_char,
    pub(crate) timestamp: i64,
    pub(crate) state: i32,
    pub(crate) id: u32,
    pub(crate) fingerprint: *mut libc::c_char,
    pub(crate) invitenumber: *mut libc::c_char,
    pub(crate) auth: *mut libc::c_char,
}

impl Lot {
    pub fn new() -> Self {
        Lot {
            text1_meaning: 0,
            text1: std::ptr::null_mut(),
            text2: std::ptr::null_mut(),
            timestamp: 0,
            state: 0,
            id: 0,
            fingerprint: std::ptr::null_mut(),
            invitenumber: std::ptr::null_mut(),
            auth: std::ptr::null_mut(),
        }
    }
    pub unsafe fn get_text1(&self) -> *mut libc::c_char {
        dc_strdup_keep_null(self.text1)
    }

    pub unsafe fn get_text2(&self) -> *mut libc::c_char {
        dc_strdup_keep_null(self.text2)
    }

    pub fn get_text1_meaning(&self) -> i32 {
        self.text1_meaning
    }

    pub fn get_state(&self) -> i32 {
        self.state
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    /* library-internal */
    /* in practice, the user additionally cuts the string himself pixel-accurate */
    pub unsafe fn fill(
        &mut self,
        msg: *mut dc_msg_t,
        chat: &Chat,
        contact: Option<&Contact>,
        context: &Context,
    ) {
        if msg.is_null() {
            return;
        }
        if (*msg).state == 19i32 {
            self.text1 = context.stock_str(StockMessage::Draft).strdup();
            self.text1_meaning = 1i32
        } else if (*msg).from_id == 1i32 as libc::c_uint {
            if 0 != dc_msg_is_info(msg) || chat.is_self_talk() {
                self.text1 = 0 as *mut libc::c_char;
                self.text1_meaning = 0i32
            } else {
                self.text1 = context.stock_str(StockMessage::SelfMsg).strdup();
                self.text1_meaning = 3i32
            }
        } else if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
            if 0 != dc_msg_is_info(msg) || contact.is_none() {
                self.text1 = 0 as *mut libc::c_char;
                self.text1_meaning = 0i32
            } else {
                if chat.id == 1 {
                    if let Some(contact) = contact {
                        self.text1 = contact.get_display_name().strdup();
                    } else {
                        self.text1 = std::ptr::null_mut();
                    }
                } else {
                    if let Some(contact) = contact {
                        self.text1 = contact.get_first_name().strdup();
                    } else {
                        self.text1 = std::ptr::null_mut();
                    }
                }
                self.text1_meaning = 2i32;
            }
        }

        self.text2 = dc_msg_get_summarytext_by_raw(
            (*msg).type_0,
            (*msg).text.as_ref(),
            &mut (*msg).param,
            160,
            context,
        )
        .strdup();

        self.timestamp = dc_msg_get_timestamp(msg);
        self.state = (*msg).state;
    }
}

impl Drop for Lot {
    fn drop(&mut self) {
        unsafe {
            free(self.text1.cast());
            free(self.text2.cast());
            free(self.fingerprint.cast());
            free(self.invitenumber.cast());
            free(self.auth.cast());
        }
    }
}
