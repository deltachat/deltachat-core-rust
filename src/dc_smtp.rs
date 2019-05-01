use libc;

use crate::constants::Event;
use crate::dc_context::dc_context_t;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_oauth2::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_smtp_t {
    pub etpan: *mut mailsmtp,
    pub from: *mut libc::c_char,
    pub esmtp: libc::c_int,
    pub log_connect_errors: libc::c_int,
    // TODO: Remvoe
    pub context: *mut dc_context_t,
    pub error: *mut libc::c_char,
    pub error_etpan: libc::c_int,
}

pub fn dc_smtp_new() -> dc_smtp_t {
    dc_smtp_t {
        etpan: std::ptr::null_mut(),
        from: std::ptr::null_mut(),
        esmtp: 0,
        log_connect_errors: 1,
        context: std::ptr::null_mut(),
        error: std::ptr::null_mut(),
        error_etpan: 0,
    }
}

pub unsafe fn dc_smtp_unref(smtp: &mut dc_smtp_t) {
    dc_smtp_disconnect(smtp);
    free(smtp.from as *mut libc::c_void);
    free(smtp.error as *mut libc::c_void);
    free(smtp as *mut libc::c_void);
}

pub unsafe fn dc_smtp_disconnect(smtp: &mut dc_smtp_t) {
    if !smtp.etpan.is_null() {
        mailsmtp_free(smtp.etpan);
        smtp.etpan = 0 as *mut mailsmtp;
    }
}

pub unsafe fn dc_smtp_is_connected(smtp: &dc_smtp_t) -> libc::c_int {
    if !smtp.etpan.is_null() {
        1
    } else {
        0
    }
}

pub unsafe fn dc_smtp_connect(smtp: &mut dc_smtp_t, lp: *const dc_loginparam_t) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut try_esmtp: libc::c_int = 0;
    if lp.is_null() {
        return 0;
    }

    if !smtp.etpan.is_null() {
        dc_log_warning(
            smtp.context,
            0,
            b"SMTP already connected.\x00" as *const u8 as *const libc::c_char,
        );
        success = 1;
    } else if (*lp).addr.is_null() || (*lp).send_server.is_null() || (*lp).send_port == 0 {
        dc_log_event_seq(
            smtp.context,
            Event::ERROR_NETWORK,
            &mut smtp.log_connect_errors as *mut libc::c_int,
            b"SMTP bad parameters.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        free(smtp.from as *mut libc::c_void);
        smtp.from = dc_strdup((*lp).addr);
        smtp.etpan = mailsmtp_new(0 as size_t, None);
        if smtp.etpan.is_null() {
            dc_log_error(
                smtp.context,
                0,
                b"SMTP-object creation failed.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            mailsmtp_set_timeout(smtp.etpan, 10 as time_t);
            mailsmtp_set_progress_callback(
                smtp.etpan,
                Some(body_progress),
                smtp as *mut libc::c_void,
            );
            /* connect to SMTP server */
            if 0 != (*lp).server_flags & (0x10000 | 0x40000) {
                r = mailsmtp_socket_connect(
                    smtp.etpan,
                    (*lp).send_server,
                    (*lp).send_port as uint16_t,
                );
                if r != MAILSMTP_NO_ERROR as libc::c_int {
                    dc_log_event_seq(
                        smtp.context,
                        Event::ERROR_NETWORK,
                        &mut smtp.log_connect_errors as *mut libc::c_int,
                        b"SMTP-Socket connection to %s:%i failed (%s)\x00" as *const u8
                            as *const libc::c_char,
                        (*lp).send_server,
                        (*lp).send_port as libc::c_int,
                        mailsmtp_strerror(r),
                    );
                    current_block = 12512295087047028901;
                } else {
                    current_block = 10043043949733653460;
                }
            } else {
                r = mailsmtp_ssl_connect(
                    smtp.etpan,
                    (*lp).send_server,
                    (*lp).send_port as uint16_t,
                );
                if r != MAILSMTP_NO_ERROR as libc::c_int {
                    dc_log_event_seq(
                        smtp.context,
                        Event::ERROR_NETWORK,
                        &mut smtp.log_connect_errors as *mut libc::c_int,
                        b"SMTP-SSL connection to %s:%i failed (%s)\x00" as *const u8
                            as *const libc::c_char,
                        (*lp).send_server,
                        (*lp).send_port as libc::c_int,
                        mailsmtp_strerror(r),
                    );
                    current_block = 12512295087047028901;
                } else {
                    current_block = 10043043949733653460;
                }
            }
            match current_block {
                12512295087047028901 => {}
                _ => {
                    try_esmtp = 1;
                    smtp.esmtp = 0;
                    if 0 != try_esmtp && {
                        r = mailesmtp_ehlo(smtp.etpan);
                        r == MAILSMTP_NO_ERROR as libc::c_int
                    } {
                        smtp.esmtp = 1
                    } else if 0 == try_esmtp || r == MAILSMTP_ERROR_NOT_IMPLEMENTED as libc::c_int {
                        r = mailsmtp_helo(smtp.etpan)
                    }
                    if r != MAILSMTP_NO_ERROR as libc::c_int {
                        dc_log_event_seq(
                            smtp.context,
                            Event::ERROR_NETWORK,
                            &mut smtp.log_connect_errors as *mut libc::c_int,
                            b"SMTP-helo failed (%s)\x00" as *const u8 as *const libc::c_char,
                            mailsmtp_strerror(r),
                        );
                    } else {
                        if 0 != (*lp).server_flags & 0x10000 {
                            r = mailsmtp_socket_starttls(smtp.etpan);
                            if r != MAILSMTP_NO_ERROR as libc::c_int {
                                dc_log_event_seq(
                                    smtp.context,
                                    Event::ERROR_NETWORK,
                                    &mut smtp.log_connect_errors as *mut libc::c_int,
                                    b"SMTP-STARTTLS failed (%s)\x00" as *const u8
                                        as *const libc::c_char,
                                    mailsmtp_strerror(r),
                                );
                                current_block = 12512295087047028901;
                            } else {
                                smtp.esmtp = 0;
                                if 0 != try_esmtp && {
                                    r = mailesmtp_ehlo(smtp.etpan);
                                    r == MAILSMTP_NO_ERROR as libc::c_int
                                } {
                                    smtp.esmtp = 1
                                } else if 0 == try_esmtp
                                    || r == MAILSMTP_ERROR_NOT_IMPLEMENTED as libc::c_int
                                {
                                    r = mailsmtp_helo(smtp.etpan)
                                }
                                if r != MAILSMTP_NO_ERROR as libc::c_int {
                                    dc_log_event_seq(
                                        smtp.context,
                                        Event::ERROR_NETWORK,
                                        &mut smtp.log_connect_errors as *mut libc::c_int,
                                        b"SMTP-helo failed (%s)\x00" as *const u8
                                            as *const libc::c_char,
                                        mailsmtp_strerror(r),
                                    );
                                    current_block = 12512295087047028901;
                                } else {
                                    dc_log_info(
                                        smtp.context,
                                        0,
                                        b"SMTP-server %s:%i STARTTLS-connected.\x00" as *const u8
                                            as *const libc::c_char,
                                        (*lp).send_server,
                                        (*lp).send_port as libc::c_int,
                                    );
                                    current_block = 5892776923941496671;
                                }
                            }
                        } else {
                            if 0 != (*lp).server_flags & 0x40000 {
                                dc_log_info(
                                    smtp.context,
                                    0,
                                    b"SMTP-server %s:%i connected.\x00" as *const u8
                                        as *const libc::c_char,
                                    (*lp).send_server,
                                    (*lp).send_port as libc::c_int,
                                );
                            } else {
                                dc_log_info(
                                    smtp.context,
                                    0,
                                    b"SMTP-server %s:%i SSL-connected.\x00" as *const u8
                                        as *const libc::c_char,
                                    (*lp).send_server,
                                    (*lp).send_port as libc::c_int,
                                );
                            }
                            current_block = 5892776923941496671;
                        }
                        match current_block {
                            12512295087047028901 => {}
                            _ => {
                                if !(*lp).send_user.is_null() {
                                    if 0 != (*lp).server_flags & 0x2 {
                                        dc_log_info(
                                            smtp.context,
                                            0,
                                            b"SMTP-OAuth2 connect...\x00" as *const u8
                                                as *const libc::c_char,
                                        );
                                        let mut access_token: *mut libc::c_char =
                                            dc_get_oauth2_access_token(
                                                smtp.context,
                                                (*lp).addr,
                                                (*lp).send_pw,
                                                0,
                                            );
                                        r = mailsmtp_oauth2_authenticate(
                                            smtp.etpan,
                                            (*lp).send_user,
                                            access_token,
                                        );
                                        if r != MAILSMTP_NO_ERROR as libc::c_int {
                                            free(access_token as *mut libc::c_void);
                                            access_token = dc_get_oauth2_access_token(
                                                smtp.context,
                                                (*lp).addr,
                                                (*lp).send_pw,
                                                0x1,
                                            );
                                            r = mailsmtp_oauth2_authenticate(
                                                smtp.etpan,
                                                (*lp).send_user,
                                                access_token,
                                            )
                                        }
                                        free(access_token as *mut libc::c_void);
                                        current_block = 15462640364611497761;
                                    } else {
                                        r = mailsmtp_auth(
                                            smtp.etpan,
                                            (*lp).send_user,
                                            (*lp).send_pw,
                                        );
                                        if r != MAILSMTP_NO_ERROR as libc::c_int {
                                            /*
                                             * There are some Mailservers which do not correclty implement PLAIN auth (hMail)
                                             * So here we try a workaround. See https://github.com/deltachat/deltachat-android/issues/67
                                             */
                                            if 0 != (*smtp.etpan).auth
                                                & MAILSMTP_AUTH_PLAIN as libc::c_int
                                            {
                                                dc_log_info(
                                                    smtp.context,
                                                    0,
                                                    b"Trying SMTP-Login workaround \"%s\"...\x00"
                                                        as *const u8
                                                        as *const libc::c_char,
                                                    (*lp).send_user,
                                                );
                                                let mut err: libc::c_int = 0;
                                                let mut hostname: [libc::c_char; 513] = [0; 513];
                                                err = gethostname(
                                                    hostname.as_mut_ptr(),
                                                    ::std::mem::size_of::<[libc::c_char; 513]>(),
                                                );
                                                if err < 0 {
                                                    dc_log_error(
                                                        smtp.context,
                                                        0,
                                                        b"SMTP-Login: Cannot get hostname.\x00"
                                                            as *const u8
                                                            as *const libc::c_char,
                                                    );
                                                    current_block = 12512295087047028901;
                                                } else {
                                                    r = mailesmtp_auth_sasl(
                                                        smtp.etpan,
                                                        b"PLAIN\x00" as *const u8
                                                            as *const libc::c_char,
                                                        hostname.as_mut_ptr(),
                                                        0 as *const libc::c_char,
                                                        0 as *const libc::c_char,
                                                        0 as *const libc::c_char,
                                                        (*lp).send_user,
                                                        (*lp).send_pw,
                                                        0 as *const libc::c_char,
                                                    );
                                                    current_block = 15462640364611497761;
                                                }
                                            } else {
                                                current_block = 15462640364611497761;
                                            }
                                        } else {
                                            current_block = 15462640364611497761;
                                        }
                                    }
                                    match current_block {
                                        12512295087047028901 => {}
                                        _ => {
                                            if r != MAILSMTP_NO_ERROR as libc::c_int {
                                                dc_log_event_seq(
                                                    smtp.context,
                                                    Event::ERROR_NETWORK,
                                                    &mut smtp.log_connect_errors
                                                        as *mut libc::c_int,
                                                    b"SMTP-login failed for user %s (%s)\x00"
                                                        as *const u8
                                                        as *const libc::c_char,
                                                    (*lp).send_user,
                                                    mailsmtp_strerror(r),
                                                );
                                                current_block = 12512295087047028901;
                                            } else {
                                                dc_log_event(
                                                    smtp.context,
                                                    Event::SMTP_CONNECTED,
                                                    0,
                                                    b"SMTP-login as %s ok.\x00" as *const u8
                                                        as *const libc::c_char,
                                                    (*lp).send_user,
                                                );
                                                current_block = 3736434875406665187;
                                            }
                                        }
                                    }
                                } else {
                                    current_block = 3736434875406665187;
                                }
                                match current_block {
                                    12512295087047028901 => {}
                                    _ => success = 1,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if 0 == success {
        if !smtp.etpan.is_null() {
            mailsmtp_free(smtp.etpan);
            smtp.etpan = 0 as *mut mailsmtp
        }
    }
    success
}

unsafe extern "C" fn body_progress(
    _current: size_t,
    _maximum: size_t,
    _user_data: *mut libc::c_void,
) {
}

pub unsafe fn dc_smtp_send_msg(
    smtp: &mut dc_smtp_t,
    recipients: *const clist,
    data_not_terminated: *const libc::c_char,
    data_bytes: size_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut iter: *mut clistiter = 0 as *mut clistiter;
    if recipients.is_null()
        || (*recipients).count == 0
        || data_not_terminated.is_null()
        || data_bytes == 0
    {
        success = 1
    } else if !smtp.etpan.is_null() {
        // set source
        // the `etPanSMTPTest` is the ENVID from RFC 3461 (SMTP DSNs), we should probably replace it by a random value
        r = if 0 != smtp.esmtp {
            mailesmtp_mail(
                smtp.etpan,
                smtp.from,
                1,
                b"etPanSMTPTest\x00" as *const u8 as *const libc::c_char,
            )
        } else {
            mailsmtp_mail(smtp.etpan, smtp.from)
        };
        if r != MAILSMTP_NO_ERROR as libc::c_int {
            log_error(
                smtp,
                b"SMTP failed to start message\x00" as *const u8 as *const libc::c_char,
                r,
            );
        } else {
            // set recipients
            // if the recipient is on the same server, this may fail at once.
            // TODO: question is what to do if one recipient in a group fails
            iter = (*recipients).first;
            loop {
                if iter.is_null() {
                    current_block = 12039483399334584727;
                    break;
                }
                let mut rcpt: *const libc::c_char = (if !iter.is_null() {
                    (*iter).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char;
                r = if 0 != smtp.esmtp {
                    mailesmtp_rcpt(smtp.etpan, rcpt, 2 | 4, 0 as *const libc::c_char)
                } else {
                    mailsmtp_rcpt(smtp.etpan, rcpt)
                };
                if r != MAILSMTP_NO_ERROR as libc::c_int {
                    log_error(
                        smtp,
                        b"SMTP failed to add recipient\x00" as *const u8 as *const libc::c_char,
                        r,
                    );
                    current_block = 5498835644851925448;
                    break;
                } else {
                    iter = if !iter.is_null() {
                        (*iter).next
                    } else {
                        0 as *mut clistcell_s
                    }
                }
            }
            match current_block {
                5498835644851925448 => {}
                _ => {
                    // message
                    r = mailsmtp_data(smtp.etpan);
                    if r != MAILSMTP_NO_ERROR as libc::c_int {
                        log_error(
                            smtp,
                            b"SMTP failed to set data\x00" as *const u8 as *const libc::c_char,
                            r,
                        );
                    } else {
                        r = mailsmtp_data_message(smtp.etpan, data_not_terminated, data_bytes);
                        if r != MAILSMTP_NO_ERROR as libc::c_int {
                            log_error(
                                smtp,
                                b"SMTP failed to send message\x00" as *const u8
                                    as *const libc::c_char,
                                r,
                            );
                        } else {
                            dc_log_event(
                                smtp.context,
                                Event::SMTP_MESSAGE_SENT,
                                0,
                                b"Message was sent to SMTP server\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            success = 1;
                        }
                    }
                }
            }
        }
    }

    success
}

unsafe fn log_error(smtp: &mut dc_smtp_t, what_failed: *const libc::c_char, r: libc::c_int) {
    let mut error_msg: *mut libc::c_char = dc_mprintf(
        b"%s: %s: %s\x00" as *const u8 as *const libc::c_char,
        what_failed,
        mailsmtp_strerror(r),
        (*smtp.etpan).response,
    );
    dc_log_warning(
        smtp.context,
        0,
        b"%s\x00" as *const u8 as *const libc::c_char,
        error_msg,
    );
    free(smtp.error as *mut libc::c_void);
    smtp.error = error_msg;
    smtp.error_etpan = r;
}
