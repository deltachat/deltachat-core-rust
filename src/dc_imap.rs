use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

use libc;

use crate::constants::Event;
use crate::dc_context::dc_context_t;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_oauth2::*;
use crate::dc_stock::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[repr(C)]
pub struct dc_imap_t {
    pub addr: *mut libc::c_char,
    pub imap_server: *mut libc::c_char,
    pub imap_port: i32,
    pub imap_user: *mut libc::c_char,
    pub imap_pw: *mut libc::c_char,
    pub server_flags: i32,
    pub connected: i32,
    pub etpan: *mut mailimap,
    pub idle_set_up: i32,
    pub selected_folder: *mut libc::c_char,
    pub selected_folder_needs_expunge: i32,
    pub should_reconnect: i32,
    pub can_idle: i32,
    pub has_xlist: i32,
    pub imap_delimiter: libc::c_char,
    pub watch_folder: *mut libc::c_char,
    pub watch: Arc<(Mutex<bool>, Condvar)>,
    pub fetch_type_prefetch: *mut mailimap_fetch_type,
    pub fetch_type_body: *mut mailimap_fetch_type,
    pub fetch_type_flags: *mut mailimap_fetch_type,
    pub get_config: dc_get_config_t,
    pub set_config: dc_set_config_t,
    pub precheck_imf: dc_precheck_imf_t,
    pub receive_imf: dc_receive_imf_t,
    pub log_connect_errors: i32,
    pub skip_log_capabilities: i32,
}

pub unsafe fn dc_imap_new(
    get_config: dc_get_config_t,
    set_config: dc_set_config_t,
    precheck_imf: dc_precheck_imf_t,
    receive_imf: dc_receive_imf_t,
) -> dc_imap_t {
    let mut imap = dc_imap_t {
        addr: std::ptr::null_mut(),
        imap_server: std::ptr::null_mut(),
        imap_port: 0,
        imap_user: std::ptr::null_mut(),
        imap_pw: std::ptr::null_mut(),
        server_flags: 0,
        connected: 0,
        etpan: std::ptr::null_mut(),
        idle_set_up: 0,
        selected_folder: calloc(1, 1) as *mut libc::c_char,
        selected_folder_needs_expunge: 0,
        should_reconnect: 0,
        can_idle: 0,
        has_xlist: 0,
        imap_delimiter: 0 as libc::c_char,
        watch_folder: calloc(1, 1) as *mut libc::c_char,
        watch: Arc::new((Mutex::new(false), Condvar::new())),
        fetch_type_prefetch: mailimap_fetch_type_new_fetch_att_list_empty(),
        fetch_type_body: mailimap_fetch_type_new_fetch_att_list_empty(),
        fetch_type_flags: mailimap_fetch_type_new_fetch_att_list_empty(),
        get_config,
        set_config,
        precheck_imf,
        receive_imf,
        log_connect_errors: 1,
        skip_log_capabilities: 0,
    };
    mailimap_fetch_type_new_fetch_att_list_add(
        imap.fetch_type_prefetch,
        mailimap_fetch_att_new_uid(),
    );
    mailimap_fetch_type_new_fetch_att_list_add(
        imap.fetch_type_prefetch,
        mailimap_fetch_att_new_envelope(),
    );

    mailimap_fetch_type_new_fetch_att_list_add(
        imap.fetch_type_body,
        mailimap_fetch_att_new_flags(),
    );
    mailimap_fetch_type_new_fetch_att_list_add(
        imap.fetch_type_body,
        mailimap_fetch_att_new_body_peek_section(mailimap_section_new(
            0 as *mut mailimap_section_spec,
        )),
    );
    mailimap_fetch_type_new_fetch_att_list_add(
        imap.fetch_type_flags,
        mailimap_fetch_att_new_flags(),
    );

    imap
}

pub unsafe fn dc_imap_unref(context: &dc_context_t, imap: &mut dc_imap_t) {
    dc_imap_disconnect(context, imap);
    free(imap.watch_folder as *mut libc::c_void);
    free(imap.selected_folder as *mut libc::c_void);
    if !imap.fetch_type_prefetch.is_null() {
        mailimap_fetch_type_free(imap.fetch_type_prefetch);
    }
    if !imap.fetch_type_body.is_null() {
        mailimap_fetch_type_free(imap.fetch_type_body);
    }
    if !imap.fetch_type_flags.is_null() {
        mailimap_fetch_type_free(imap.fetch_type_flags);
    }
}

pub unsafe fn dc_imap_disconnect(context: &dc_context_t, imap: &mut dc_imap_t) {
    if 0 != imap.connected {
        unsetup_handle(context, imap);
        free_connect_param(imap);
        imap.connected = 0
    };
}

/* ******************************************************************************
 * Connect/Disconnect
 ******************************************************************************/

unsafe fn free_connect_param(imap: &mut dc_imap_t) {
    free(imap.addr as *mut libc::c_void);
    imap.addr = 0 as *mut libc::c_char;
    free(imap.imap_server as *mut libc::c_void);
    imap.imap_server = 0 as *mut libc::c_char;
    free(imap.imap_user as *mut libc::c_void);
    imap.imap_user = 0 as *mut libc::c_char;
    free(imap.imap_pw as *mut libc::c_void);
    imap.imap_pw = 0 as *mut libc::c_char;
    *imap.watch_folder.offset(0isize) = 0 as libc::c_char;
    *imap.selected_folder.offset(0isize) = 0 as libc::c_char;
    imap.imap_port = 0;
    imap.can_idle = 0;
    imap.has_xlist = 0;
}

unsafe fn unsetup_handle(context: &dc_context_t, imap: &mut dc_imap_t) {
    if !imap.etpan.is_null() {
        if 0 != imap.idle_set_up {
            mailstream_unsetup_idle((*imap.etpan).imap_stream);
            imap.idle_set_up = 0
        }
        if !(*imap.etpan).imap_stream.is_null() {
            mailstream_close((*imap.etpan).imap_stream);
            (*imap.etpan).imap_stream = 0 as *mut mailstream
        }
        mailimap_free(imap.etpan);
        imap.etpan = 0 as *mut mailimap;
        dc_log_info(
            context,
            0,
            b"IMAP disconnected.\x00" as *const u8 as *const libc::c_char,
        );
    }
    *imap.selected_folder.offset(0isize) = 0 as libc::c_char;
}

pub unsafe fn dc_imap_connect(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    lp: *const dc_loginparam_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0;
    if lp.is_null()
        || (*lp).mail_server.is_null()
        || (*lp).mail_user.is_null()
        || (*lp).mail_pw.is_null()
    {
        return 0;
    }
    if 0 != imap.connected {
        success = 1
    } else {
        imap.addr = dc_strdup((*lp).addr);
        imap.imap_server = dc_strdup((*lp).mail_server);
        imap.imap_port = (*lp).mail_port as libc::c_int;
        imap.imap_user = dc_strdup((*lp).mail_user);
        imap.imap_pw = dc_strdup((*lp).mail_pw);
        imap.server_flags = (*lp).server_flags;
        if !(0 == setup_handle_if_needed(context, imap)) {
            imap.can_idle = mailimap_has_idle(imap.etpan);
            imap.has_xlist = mailimap_has_xlist(imap.etpan);
            imap.can_idle = 0;
            if 0 == imap.skip_log_capabilities
                && !(*imap.etpan).imap_connection_info.is_null()
                && !(*(*imap.etpan).imap_connection_info)
                    .imap_capability
                    .is_null()
            {
                imap.skip_log_capabilities = 1;
                let mut capinfostr: dc_strbuilder_t = dc_strbuilder_t {
                    buf: 0 as *mut libc::c_char,
                    allocated: 0,
                    free: 0,
                    eos: 0 as *mut libc::c_char,
                };
                dc_strbuilder_init(&mut capinfostr, 0);
                let mut list: *mut clist =
                    (*(*(*imap.etpan).imap_connection_info).imap_capability).cap_list;
                if !list.is_null() {
                    let mut cur: *mut clistiter = 0 as *mut clistiter;
                    cur = (*list).first;
                    while !cur.is_null() {
                        let mut cap: *mut mailimap_capability = (if !cur.is_null() {
                            (*cur).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *mut mailimap_capability;
                        if !cap.is_null()
                            && (*cap).cap_type == MAILIMAP_CAPABILITY_NAME as libc::c_int
                        {
                            dc_strbuilder_cat(
                                &mut capinfostr,
                                b" \x00" as *const u8 as *const libc::c_char,
                            );
                            dc_strbuilder_cat(&mut capinfostr, (*cap).cap_data.cap_name);
                        }
                        cur = if !cur.is_null() {
                            (*cur).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                }
                dc_log_info(
                    context,
                    0,
                    b"IMAP-capabilities:%s\x00" as *const u8 as *const libc::c_char,
                    capinfostr.buf,
                );
                free(capinfostr.buf as *mut libc::c_void);
            }
            imap.connected = 1;
            success = 1
        }
    }
    if success == 0 {
        unsetup_handle(context, imap);
        free_connect_param(imap);
    }
    success
}

unsafe fn setup_handle_if_needed(context: &dc_context_t, imap: &mut dc_imap_t) -> libc::c_int {
    let mut current_block: u64;
    let mut r: libc::c_int = 0;
    let mut success: libc::c_int = 0;
    if !(imap.imap_server.is_null()) {
        if 0 != imap.should_reconnect {
            unsetup_handle(context, imap);
        }
        if !imap.etpan.is_null() {
            success = 1
        } else {
            imap.etpan = mailimap_new(0 as size_t, None);
            mailimap_set_timeout(imap.etpan, 10 as time_t);
            if 0 != imap.server_flags & (0x100 | 0x400) {
                r = mailimap_socket_connect(
                    imap.etpan,
                    imap.imap_server,
                    imap.imap_port as uint16_t,
                );
                if 0 != dc_imap_is_error(context, imap, r) {
                    dc_log_event_seq(
                        context,
                        Event::ERROR_NETWORK,
                        &mut imap.log_connect_errors as *mut libc::c_int,
                        b"Could not connect to IMAP-server %s:%i. (Error #%i)\x00" as *const u8
                            as *const libc::c_char,
                        imap.imap_server,
                        imap.imap_port as libc::c_int,
                        r as libc::c_int,
                    );
                    current_block = 15811161807000851472;
                } else if 0 != imap.server_flags & 0x100 {
                    r = mailimap_socket_starttls(imap.etpan);
                    if 0 != dc_imap_is_error(context, imap, r) {
                        dc_log_event_seq(context, Event::ERROR_NETWORK,
                                         &mut imap.log_connect_errors as
                                             *mut libc::c_int,
                                         b"Could not connect to IMAP-server %s:%i using STARTTLS. (Error #%i)\x00"
                                             as *const u8 as
                                             *const libc::c_char,
                                         imap.imap_server,
                                         imap.imap_port as libc::c_int,
                                         r as libc::c_int);
                        current_block = 15811161807000851472;
                    } else {
                        dc_log_info(
                            context,
                            0,
                            b"IMAP-server %s:%i STARTTLS-connected.\x00" as *const u8
                                as *const libc::c_char,
                            imap.imap_server,
                            imap.imap_port as libc::c_int,
                        );
                        current_block = 14763689060501151050;
                    }
                } else {
                    dc_log_info(
                        context,
                        0,
                        b"IMAP-server %s:%i connected.\x00" as *const u8 as *const libc::c_char,
                        imap.imap_server,
                        imap.imap_port as libc::c_int,
                    );
                    current_block = 14763689060501151050;
                }
            } else {
                r = mailimap_ssl_connect(imap.etpan, imap.imap_server, imap.imap_port as uint16_t);
                if 0 != dc_imap_is_error(context, imap, r) {
                    dc_log_event_seq(
                        context,
                        Event::ERROR_NETWORK,
                        &mut imap.log_connect_errors as *mut libc::c_int,
                        b"Could not connect to IMAP-server %s:%i using SSL. (Error #%i)\x00"
                            as *const u8 as *const libc::c_char,
                        imap.imap_server,
                        imap.imap_port as libc::c_int,
                        r as libc::c_int,
                    );
                    current_block = 15811161807000851472;
                } else {
                    dc_log_info(
                        context,
                        0,
                        b"IMAP-server %s:%i SSL-connected.\x00" as *const u8 as *const libc::c_char,
                        imap.imap_server,
                        imap.imap_port as libc::c_int,
                    );
                    current_block = 14763689060501151050;
                }
            }
            match current_block {
                15811161807000851472 => {}
                _ => {
                    if 0 != imap.server_flags & 0x2 {
                        dc_log_info(
                            context,
                            0,
                            b"IMAP-OAuth2 connect...\x00" as *const u8 as *const libc::c_char,
                        );
                        let mut access_token: *mut libc::c_char =
                            dc_get_oauth2_access_token(context, imap.addr, imap.imap_pw, 0);
                        r = mailimap_oauth2_authenticate(imap.etpan, imap.imap_user, access_token);
                        if 0 != dc_imap_is_error(context, imap, r) {
                            free(access_token as *mut libc::c_void);
                            access_token =
                                dc_get_oauth2_access_token(context, imap.addr, imap.imap_pw, 0x1);
                            r = mailimap_oauth2_authenticate(
                                imap.etpan,
                                imap.imap_user,
                                access_token,
                            )
                        }
                        free(access_token as *mut libc::c_void);
                    } else {
                        r = mailimap_login(imap.etpan, imap.imap_user, imap.imap_pw)
                    }
                    if 0 != dc_imap_is_error(context, imap, r) {
                        let mut msg: *mut libc::c_char = get_error_msg(
                            context,
                            imap,
                            b"Cannot login\x00" as *const u8 as *const libc::c_char,
                            r,
                        );
                        dc_log_event_seq(
                            context,
                            Event::ERROR_NETWORK,
                            &mut imap.log_connect_errors as *mut libc::c_int,
                            b"%s\x00" as *const u8 as *const libc::c_char,
                            msg,
                        );
                        free(msg as *mut libc::c_void);
                    } else {
                        dc_log_event(
                            context,
                            Event::IMAP_CONNECTED,
                            0,
                            b"IMAP-login as %s ok.\x00" as *const u8 as *const libc::c_char,
                            imap.imap_user,
                        );
                        success = 1
                    }
                }
            }
        }
    }
    if success == 0 {
        unsetup_handle(context, imap);
    }
    imap.should_reconnect = 0;
    success
}

unsafe fn get_error_msg(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    what_failed: *const libc::c_char,
    code: libc::c_int,
) -> *mut libc::c_char {
    let mut stock: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut msg: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut msg, 1000);
    match code {
        28 => {
            stock = dc_stock_str_repl_string(context, 60, imap.imap_user);
            dc_strbuilder_cat(&mut msg, stock);
        }
        _ => {
            dc_strbuilder_catf(
                &mut msg as *mut dc_strbuilder_t,
                b"%s, IMAP-error #%i\x00" as *const u8 as *const libc::c_char,
                what_failed,
                code,
            );
        }
    }
    free(stock as *mut libc::c_void);
    stock = 0 as *mut libc::c_char;
    if !(*imap.etpan).imap_response.is_null() {
        dc_strbuilder_cat(&mut msg, b"\n\n\x00" as *const u8 as *const libc::c_char);
        stock =
            dc_stock_str_repl_string2(context, 61, imap.imap_server, (*imap.etpan).imap_response);
        dc_strbuilder_cat(&mut msg, stock);
    }
    free(stock as *mut libc::c_void);
    stock = 0 as *mut libc::c_char;
    msg.buf
}

pub unsafe fn dc_imap_is_error(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    code: libc::c_int,
) -> libc::c_int {
    if code == MAILIMAP_NO_ERROR as libc::c_int
        || code == MAILIMAP_NO_ERROR_AUTHENTICATED as libc::c_int
        || code == MAILIMAP_NO_ERROR_NON_AUTHENTICATED as libc::c_int
    {
        return 0;
    }
    if code == MAILIMAP_ERROR_STREAM as libc::c_int || code == MAILIMAP_ERROR_PARSE as libc::c_int {
        dc_log_info(
            context,
            0,
            b"IMAP stream lost; we\'ll reconnect soon.\x00" as *const u8 as *const libc::c_char,
        );
        imap.should_reconnect = 1
    }
    1
}

pub unsafe extern "C" fn dc_imap_set_watch_folder(
    imap: &mut dc_imap_t,
    watch_folder: *const libc::c_char,
) {
    if watch_folder.is_null() {
        return;
    }
    free(imap.watch_folder as *mut libc::c_void);
    imap.watch_folder = dc_strdup(watch_folder);
}

pub unsafe fn dc_imap_is_connected(imap: &dc_imap_t) -> libc::c_int {
    (0 != imap.connected) as libc::c_int
}

pub unsafe fn dc_imap_fetch(context: &dc_context_t, imap: &mut dc_imap_t) -> libc::c_int {
    let mut success = 0;
    if 0 != imap.connected {
        setup_handle_if_needed(context, imap);
        while fetch_from_single_folder(context, imap, imap.watch_folder) > 0 {}
        success = 1;
    }
    success
}

unsafe fn fetch_from_single_folder(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut r: libc::c_int = 0;
    let mut uidvalidity: uint32_t = 0 as uint32_t;
    let mut lastseenuid: uint32_t = 0 as uint32_t;
    let mut new_lastseenuid: uint32_t = 0 as uint32_t;
    let mut fetch_result: *mut clist = 0 as *mut clist;
    let mut read_cnt: size_t = 0 as size_t;
    let mut read_errors: size_t = 0 as size_t;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut set: *mut mailimap_set = 0 as *mut mailimap_set;
    if imap.etpan.is_null() {
        dc_log_info(
            context,
            0,
            b"Cannot fetch from \"%s\" - not connected.\x00" as *const u8 as *const libc::c_char,
            folder,
        );
    } else if select_folder(context, imap, folder) == 0 {
        dc_log_warning(
            context,
            0,
            b"Cannot select folder %s for fetching.\x00" as *const u8 as *const libc::c_char,
            folder,
        );
    } else {
        get_config_lastseenuid(context, imap, folder, &mut uidvalidity, &mut lastseenuid);
        if uidvalidity != (*(*imap.etpan).imap_selection_info).sel_uidvalidity {
            /* first time this folder is selected or UIDVALIDITY has changed, init lastseenuid and save it to config */
            if (*(*imap.etpan).imap_selection_info).sel_uidvalidity <= 0 as libc::c_uint {
                dc_log_error(
                    context,
                    0,
                    b"Cannot get UIDVALIDITY for folder \"%s\".\x00" as *const u8
                        as *const libc::c_char,
                    folder,
                );
                current_block = 17288151659885296046;
            } else {
                if 0 != (*(*imap.etpan).imap_selection_info).sel_has_exists() {
                    if (*(*imap.etpan).imap_selection_info).sel_exists <= 0 as libc::c_uint {
                        dc_log_info(
                            context,
                            0,
                            b"Folder \"%s\" is empty.\x00" as *const u8 as *const libc::c_char,
                            folder,
                        );
                        if (*(*imap.etpan).imap_selection_info).sel_exists == 0 as libc::c_uint {
                            set_config_lastseenuid(
                                context,
                                imap,
                                folder,
                                (*(*imap.etpan).imap_selection_info).sel_uidvalidity,
                                0 as uint32_t,
                            );
                        }
                        current_block = 17288151659885296046;
                    } else {
                        set = mailimap_set_new_single(
                            (*(*imap.etpan).imap_selection_info).sel_exists,
                        );
                        current_block = 11057878835866523405;
                    }
                } else {
                    dc_log_info(
                        context,
                        0,
                        b"EXISTS is missing for folder \"%s\", using fallback.\x00" as *const u8
                            as *const libc::c_char,
                        folder,
                    );
                    set = mailimap_set_new_single(0 as uint32_t);
                    current_block = 11057878835866523405;
                }
                match current_block {
                    17288151659885296046 => {}
                    _ => {
                        r = mailimap_fetch(
                            imap.etpan,
                            set,
                            imap.fetch_type_prefetch,
                            &mut fetch_result,
                        );
                        if !set.is_null() {
                            mailimap_set_free(set);
                            set = 0 as *mut mailimap_set
                        }
                        if 0 != dc_imap_is_error(context, imap, r) || fetch_result.is_null() {
                            fetch_result = 0 as *mut clist;
                            dc_log_info(
                                context,
                                0,
                                b"No result returned for folder \"%s\".\x00" as *const u8
                                    as *const libc::c_char,
                                folder,
                            );
                            /* this might happen if the mailbox is empty an EXISTS does not work */
                            current_block = 17288151659885296046;
                        } else {
                            cur = (*fetch_result).first;
                            if cur.is_null() {
                                dc_log_info(
                                    context,
                                    0,
                                    b"Empty result returned for folder \"%s\".\x00" as *const u8
                                        as *const libc::c_char,
                                    folder,
                                );
                                /* this might happen if the mailbox is empty an EXISTS does not work */
                                current_block = 17288151659885296046;
                            } else {
                                let mut msg_att: *mut mailimap_msg_att = (if !cur.is_null() {
                                    (*cur).data
                                } else {
                                    0 as *mut libc::c_void
                                })
                                    as *mut mailimap_msg_att;
                                lastseenuid = peek_uid(msg_att);
                                if !fetch_result.is_null() {
                                    mailimap_fetch_list_free(fetch_result);
                                    fetch_result = 0 as *mut clist
                                }
                                if lastseenuid <= 0 as libc::c_uint {
                                    dc_log_error(
                                        context,
                                        0,
                                        b"Cannot get largest UID for folder \"%s\"\x00" as *const u8
                                            as *const libc::c_char,
                                        folder,
                                    );
                                    current_block = 17288151659885296046;
                                } else {
                                    if uidvalidity > 0 as libc::c_uint
                                        && lastseenuid > 1 as libc::c_uint
                                    {
                                        lastseenuid = (lastseenuid as libc::c_uint)
                                            .wrapping_sub(1 as libc::c_uint)
                                            as uint32_t
                                            as uint32_t
                                    }
                                    uidvalidity =
                                        (*(*imap.etpan).imap_selection_info).sel_uidvalidity;
                                    set_config_lastseenuid(
                                        context,
                                        imap,
                                        folder,
                                        uidvalidity,
                                        lastseenuid,
                                    );
                                    dc_log_info(
                                        context,
                                        0,
                                        b"lastseenuid initialized to %i for %s@%i\x00" as *const u8
                                            as *const libc::c_char,
                                        lastseenuid as libc::c_int,
                                        folder,
                                        uidvalidity as libc::c_int,
                                    );
                                    current_block = 2516253395664191498;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            current_block = 2516253395664191498;
        }
        match current_block {
            17288151659885296046 => {}
            _ => {
                set = mailimap_set_new_interval(
                    lastseenuid.wrapping_add(1 as libc::c_uint),
                    0 as uint32_t,
                );
                r = mailimap_uid_fetch(
                    imap.etpan,
                    set,
                    imap.fetch_type_prefetch,
                    &mut fetch_result,
                );
                if !set.is_null() {
                    mailimap_set_free(set);
                    set = 0 as *mut mailimap_set
                }
                if 0 != dc_imap_is_error(context, imap, r) || fetch_result.is_null() {
                    fetch_result = 0 as *mut clist;
                    if r == MAILIMAP_ERROR_PROTOCOL as libc::c_int {
                        dc_log_info(
                            context,
                            0,
                            b"Folder \"%s\" is empty\x00" as *const u8 as *const libc::c_char,
                            folder,
                        );
                    } else {
                        /* the folder is simply empty, this is no error */
                        dc_log_warning(
                            context,
                            0,
                            b"Cannot fetch message list from folder \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            folder,
                        );
                    }
                } else {
                    cur = (*fetch_result).first;
                    while !cur.is_null() {
                        let mut msg_att_0: *mut mailimap_msg_att = (if !cur.is_null() {
                            (*cur).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *mut mailimap_msg_att;
                        let mut cur_uid: uint32_t = peek_uid(msg_att_0);
                        if cur_uid > lastseenuid {
                            let mut rfc724_mid: *mut libc::c_char =
                                unquote_rfc724_mid(peek_rfc724_mid(msg_att_0));
                            read_cnt = read_cnt.wrapping_add(1);
                            if 0 == imap.precheck_imf.expect("non-null function pointer")(
                                context, rfc724_mid, folder, cur_uid,
                            ) {
                                if fetch_single_msg(context, imap, folder, cur_uid) == 0 {
                                    dc_log_info(context, 0,
                                                b"Read error for message %s from \"%s\", trying over later.\x00"
                                                as *const u8 as
                                                *const libc::c_char,
                                                rfc724_mid, folder);
                                    read_errors = read_errors.wrapping_add(1)
                                }
                            } else {
                                dc_log_info(
                                    context,
                                    0,
                                    b"Skipping message %s from \"%s\" by precheck.\x00" as *const u8
                                        as *const libc::c_char,
                                    rfc724_mid,
                                    folder,
                                );
                            }
                            if cur_uid > new_lastseenuid {
                                new_lastseenuid = cur_uid
                            }
                            free(rfc724_mid as *mut libc::c_void);
                        }
                        cur = if !cur.is_null() {
                            (*cur).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                    if 0 == read_errors && new_lastseenuid > 0 as libc::c_uint {
                        set_config_lastseenuid(context, imap, folder, uidvalidity, new_lastseenuid);
                    }
                }
            }
        }
    }

    /* done */
    if 0 != read_errors {
        dc_log_warning(
            context,
            0,
            b"%i mails read from \"%s\" with %i errors.\x00" as *const u8 as *const libc::c_char,
            read_cnt as libc::c_int,
            folder,
            read_errors as libc::c_int,
        );
    } else {
        dc_log_info(
            context,
            0i32,
            b"%i mails read from \"%s\".\x00" as *const u8 as *const libc::c_char,
            read_cnt as libc::c_int,
            folder,
        );
    }
    if !fetch_result.is_null() {
        mailimap_fetch_list_free(fetch_result);
        fetch_result = 0 as *mut clist
    }

    read_cnt as libc::c_int
}

unsafe fn set_config_lastseenuid(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
    uidvalidity: uint32_t,
    lastseenuid: uint32_t,
) {
    let mut key: *mut libc::c_char = dc_mprintf(
        b"imap.mailbox.%s\x00" as *const u8 as *const libc::c_char,
        folder,
    );
    let mut val: *mut libc::c_char = dc_mprintf(
        b"%lu:%lu\x00" as *const u8 as *const libc::c_char,
        uidvalidity,
        lastseenuid,
    );
    imap.set_config.expect("non-null function pointer")(context, key, val);
    free(val as *mut libc::c_void);
    free(key as *mut libc::c_void);
}

unsafe fn peek_rfc724_mid(msg_att: *mut mailimap_msg_att) -> *const libc::c_char {
    if msg_att.is_null() {
        return 0 as *const libc::c_char;
    }
    /* search the UID in a list of attributes returned by a FETCH command */
    let mut iter1: *mut clistiter = 0 as *mut clistiter;
    iter1 = (*(*msg_att).att_list).first;
    while !iter1.is_null() {
        let mut item: *mut mailimap_msg_att_item = (if !iter1.is_null() {
            (*iter1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimap_msg_att_item;
        if !item.is_null() {
            if (*item).att_type == MAILIMAP_MSG_ATT_ITEM_STATIC as libc::c_int {
                if (*(*item).att_data.att_static).att_type
                    == MAILIMAP_MSG_ATT_ENVELOPE as libc::c_int
                {
                    let mut env: *mut mailimap_envelope =
                        (*(*item).att_data.att_static).att_data.att_env;
                    if !env.is_null() && !(*env).env_message_id.is_null() {
                        return (*env).env_message_id;
                    }
                }
            }
        }
        iter1 = if !iter1.is_null() {
            (*iter1).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return 0 as *const libc::c_char;
}

unsafe fn unquote_rfc724_mid(in_0: *const libc::c_char) -> *mut libc::c_char {
    /* remove < and > from the given message id */
    let mut out: *mut libc::c_char = dc_strdup(in_0);
    let mut out_len: libc::c_int = strlen(out) as libc::c_int;
    if out_len > 2 {
        if *out.offset(0isize) as libc::c_int == '<' as i32 {
            *out.offset(0isize) = ' ' as i32 as libc::c_char
        }
        if *out.offset((out_len - 1) as isize) as libc::c_int == '>' as i32 {
            *out.offset((out_len - 1) as isize) = ' ' as i32 as libc::c_char
        }
        dc_trim(out);
    }
    out
}

/* ******************************************************************************
 * Fetch Messages
 ******************************************************************************/

unsafe fn peek_uid(msg_att: *mut mailimap_msg_att) -> uint32_t {
    /* search the UID in a list of attributes returned by a FETCH command */
    let mut iter1: *mut clistiter = 0 as *mut clistiter;
    iter1 = (*(*msg_att).att_list).first;
    while !iter1.is_null() {
        let mut item: *mut mailimap_msg_att_item = (if !iter1.is_null() {
            (*iter1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimap_msg_att_item;
        if !item.is_null() {
            if (*item).att_type == MAILIMAP_MSG_ATT_ITEM_STATIC as libc::c_int {
                if (*(*item).att_data.att_static).att_type == MAILIMAP_MSG_ATT_UID as libc::c_int {
                    return (*(*item).att_data.att_static).att_data.att_uid;
                }
            }
        }
        iter1 = if !iter1.is_null() {
            (*iter1).next
        } else {
            0 as *mut clistcell_s
        }
    }
    0 as uint32_t
}

unsafe fn fetch_single_msg(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
    server_uid: uint32_t,
) -> libc::c_int {
    let mut msg_att: *mut mailimap_msg_att = 0 as *mut mailimap_msg_att;
    /* the function returns:
        0  the caller should try over again later
    or  1  if the messages should be treated as received, the caller should not try to read the message again (even if no database entries are returned) */
    let mut msg_content: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut msg_bytes: size_t = 0 as size_t;
    let mut r: libc::c_int = 0;
    let mut retry_later: libc::c_int = 0;
    let mut deleted: libc::c_int = 0;
    let mut flags: uint32_t = 0 as uint32_t;
    let mut fetch_result: *mut clist = 0 as *mut clist;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    if !imap.etpan.is_null() {
        let mut set: *mut mailimap_set = mailimap_set_new_single(server_uid);
        r = mailimap_uid_fetch(imap.etpan, set, imap.fetch_type_body, &mut fetch_result);
        if !set.is_null() {
            mailimap_set_free(set);
            set = 0 as *mut mailimap_set
        }
        if 0 != dc_imap_is_error(context, imap, r) || fetch_result.is_null() {
            fetch_result = 0 as *mut clist;
            dc_log_warning(
                context,
                0,
                b"Error #%i on fetching message #%i from folder \"%s\"; retry=%i.\x00" as *const u8
                    as *const libc::c_char,
                r as libc::c_int,
                server_uid as libc::c_int,
                folder,
                imap.should_reconnect as libc::c_int,
            );
            if 0 != imap.should_reconnect {
                retry_later = 1
            }
        } else {
            /* this is an error that should be recovered; the caller should try over later to fetch the message again (if there is no such message, we simply get an empty result) */
            cur = (*fetch_result).first;
            if cur.is_null() {
                dc_log_warning(
                    context,
                    0,
                    b"Message #%i does not exist in folder \"%s\".\x00" as *const u8
                        as *const libc::c_char,
                    server_uid as libc::c_int,
                    folder,
                );
            } else {
                /* server response is fine, however, there is no such message, do not try to fetch the message again */
                msg_att = (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailimap_msg_att;
                peek_body(
                    msg_att,
                    &mut msg_content,
                    &mut msg_bytes,
                    &mut flags,
                    &mut deleted,
                );
                if !(msg_content.is_null() || msg_bytes <= 0 || 0 != deleted) {
                    /* dc_log_warning(imap->context, 0, "Message #%i in folder \"%s\" is empty or deleted.", (int)server_uid, folder); -- this is a quite usual situation, do not print a warning */
                    imap.receive_imf.expect("non-null function pointer")(
                        context,
                        msg_content,
                        msg_bytes,
                        folder,
                        server_uid,
                        flags,
                    );
                }
            }
        }
    }

    if !fetch_result.is_null() {
        mailimap_fetch_list_free(fetch_result);
        fetch_result = 0 as *mut clist
    }
    if 0 != retry_later {
        0
    } else {
        1
    }
}

unsafe fn peek_body(
    msg_att: *mut mailimap_msg_att,
    p_msg: *mut *mut libc::c_char,
    p_msg_bytes: *mut size_t,
    flags: *mut uint32_t,
    deleted: *mut libc::c_int,
) {
    if msg_att.is_null() {
        return;
    }
    /* search body & Co. in a list of attributes returned by a FETCH command */
    let mut iter1: *mut clistiter = 0 as *mut clistiter;
    let mut iter2: *mut clistiter = 0 as *mut clistiter;
    iter1 = (*(*msg_att).att_list).first;
    while !iter1.is_null() {
        let mut item: *mut mailimap_msg_att_item = (if !iter1.is_null() {
            (*iter1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimap_msg_att_item;
        if !item.is_null() {
            if (*item).att_type == MAILIMAP_MSG_ATT_ITEM_DYNAMIC as libc::c_int {
                if !(*(*item).att_data.att_dyn).att_list.is_null() {
                    iter2 = (*(*(*item).att_data.att_dyn).att_list).first;
                    while !iter2.is_null() {
                        let mut flag_fetch: *mut mailimap_flag_fetch = (if !iter2.is_null() {
                            (*iter2).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *mut mailimap_flag_fetch;
                        if !flag_fetch.is_null()
                            && (*flag_fetch).fl_type == MAILIMAP_FLAG_FETCH_OTHER as libc::c_int
                        {
                            let mut flag: *mut mailimap_flag = (*flag_fetch).fl_flag;
                            if !flag.is_null() {
                                if (*flag).fl_type == MAILIMAP_FLAG_SEEN as libc::c_int {
                                    *flags = (*flags as libc::c_long | 0x1) as uint32_t
                                } else if (*flag).fl_type == MAILIMAP_FLAG_DELETED as libc::c_int {
                                    *deleted = 1
                                }
                            }
                        }
                        iter2 = if !iter2.is_null() {
                            (*iter2).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                }
            } else if (*item).att_type == MAILIMAP_MSG_ATT_ITEM_STATIC as libc::c_int {
                if (*(*item).att_data.att_static).att_type
                    == MAILIMAP_MSG_ATT_BODY_SECTION as libc::c_int
                {
                    *p_msg =
                        (*(*(*item).att_data.att_static).att_data.att_body_section).sec_body_part;
                    *p_msg_bytes =
                        (*(*(*item).att_data.att_static).att_data.att_body_section).sec_length;
                }
            }
        }
        iter1 = if !iter1.is_null() {
            (*iter1).next
        } else {
            0 as *mut clistcell_s
        };
    }
}

unsafe fn get_config_lastseenuid(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
    uidvalidity: *mut uint32_t,
    lastseenuid: *mut uint32_t,
) {
    *uidvalidity = 0 as uint32_t;
    *lastseenuid = 0 as uint32_t;
    let mut key: *mut libc::c_char = dc_mprintf(
        b"imap.mailbox.%s\x00" as *const u8 as *const libc::c_char,
        folder,
    );
    let mut val1: *mut libc::c_char =
        imap.get_config.expect("non-null function pointer")(context, key, 0 as *const libc::c_char);
    let mut val2: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut val3: *mut libc::c_char = 0 as *mut libc::c_char;
    if !val1.is_null() {
        val2 = strchr(val1, ':' as i32);
        if !val2.is_null() {
            *val2 = 0 as libc::c_char;
            val2 = val2.offset(1isize);
            val3 = strchr(val2, ':' as i32);
            if !val3.is_null() {
                *val3 = 0 as libc::c_char
            }
            *uidvalidity = atol(val1) as uint32_t;
            *lastseenuid = atol(val2) as uint32_t
        }
    }
    free(val1 as *mut libc::c_void);
    free(key as *mut libc::c_void);
}

/* ******************************************************************************
 * Handle folders
 ******************************************************************************/

unsafe fn select_folder(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
) -> libc::c_int {
    if imap.etpan.is_null() {
        *imap.selected_folder.offset(0isize) = 0 as libc::c_char;
        imap.selected_folder_needs_expunge = 0;
        return 0;
    }
    if !folder.is_null()
        && 0 != *folder.offset(0isize) as libc::c_int
        && strcmp(imap.selected_folder, folder) == 0
    {
        return 1;
    }
    if 0 != imap.selected_folder_needs_expunge {
        if 0 != *imap.selected_folder.offset(0isize) {
            dc_log_info(
                context,
                0,
                b"Expunge messages in \"%s\".\x00" as *const u8 as *const libc::c_char,
                imap.selected_folder,
            );
            mailimap_close(imap.etpan);
        }
        imap.selected_folder_needs_expunge = 0
    }
    if !folder.is_null() {
        let mut r: libc::c_int = mailimap_select(imap.etpan, folder);
        if 0 != dc_imap_is_error(context, imap, r) || (*imap.etpan).imap_selection_info.is_null() {
            dc_log_info(
                context,
                0,
                b"Cannot select folder; code=%i, imap_response=%s\x00" as *const u8
                    as *const libc::c_char,
                r,
                if !(*imap.etpan).imap_response.is_null() {
                    (*imap.etpan).imap_response
                } else {
                    b"<none>\x00" as *const u8 as *const libc::c_char
                },
            );
            *imap.selected_folder.offset(0isize) = 0 as libc::c_char;
            return 0;
        }
    }
    free(imap.selected_folder as *mut libc::c_void);
    imap.selected_folder = dc_strdup(folder);
    1
}

pub unsafe fn dc_imap_idle(context: &dc_context_t, imap: &mut dc_imap_t) {
    let mut current_block: u64;
    let mut r: libc::c_int = 0;
    let mut r2: libc::c_int = 0;
    if 0 != imap.can_idle {
        setup_handle_if_needed(context, imap);
        if imap.idle_set_up == 0 && !imap.etpan.is_null() && !(*imap.etpan).imap_stream.is_null() {
            r = mailstream_setup_idle((*imap.etpan).imap_stream);
            if 0 != dc_imap_is_error(context, imap, r) {
                dc_log_warning(
                    context,
                    0,
                    b"IMAP-IDLE: Cannot setup.\x00" as *const u8 as *const libc::c_char,
                );
                fake_idle(context, imap);
                current_block = 14832935472441733737;
            } else {
                imap.idle_set_up = 1;
                current_block = 17965632435239708295;
            }
        } else {
            current_block = 17965632435239708295;
        }
        match current_block {
            14832935472441733737 => {}
            _ => {
                if 0 == imap.idle_set_up || 0 == select_folder(context, imap, imap.watch_folder) {
                    dc_log_warning(
                        context,
                        0,
                        b"IMAP-IDLE not setup.\x00" as *const u8 as *const libc::c_char,
                    );
                    fake_idle(context, imap);
                } else {
                    r = mailimap_idle(imap.etpan);
                    if 0 != dc_imap_is_error(context, imap, r) {
                        dc_log_warning(
                            context,
                            0,
                            b"IMAP-IDLE: Cannot start.\x00" as *const u8 as *const libc::c_char,
                        );
                        fake_idle(context, imap);
                    } else {
                        r = mailstream_wait_idle((*imap.etpan).imap_stream, 23 * 60);
                        r2 = mailimap_idle_done(imap.etpan);
                        if r == MAILSTREAM_IDLE_ERROR as libc::c_int
                            || r == MAILSTREAM_IDLE_CANCELLED as libc::c_int
                        {
                            dc_log_info(
                                context,
                                0,
                                b"IMAP-IDLE wait cancelled, r=%i, r2=%i; we\'ll reconnect soon.\x00"
                                    as *const u8
                                    as *const libc::c_char,
                                r,
                                r2,
                            );
                            imap.should_reconnect = 1
                        } else if r == MAILSTREAM_IDLE_INTERRUPTED as libc::c_int {
                            dc_log_info(
                                context,
                                0,
                                b"IMAP-IDLE interrupted.\x00" as *const u8 as *const libc::c_char,
                            );
                        } else if r == MAILSTREAM_IDLE_HASDATA as libc::c_int {
                            dc_log_info(
                                context,
                                0,
                                b"IMAP-IDLE has data.\x00" as *const u8 as *const libc::c_char,
                            );
                        } else if r == MAILSTREAM_IDLE_TIMEOUT as libc::c_int {
                            dc_log_info(
                                context,
                                0,
                                b"IMAP-IDLE timeout.\x00" as *const u8 as *const libc::c_char,
                            );
                        } else {
                            dc_log_warning(
                                context,
                                0,
                                b"IMAP-IDLE returns unknown value r=%i, r2=%i.\x00" as *const u8
                                    as *const libc::c_char,
                                r,
                                r2,
                            );
                        }
                    }
                }
            }
        }
    } else {
        fake_idle(context, imap);
    }
}

unsafe fn fake_idle(context: &dc_context_t, imap: &mut dc_imap_t) {
    /* Idle using timeouts. This is also needed if we're not yet configured -
    in this case, we're waiting for a configure job */
    let mut fake_idle_start_time = SystemTime::now();

    dc_log_info(
        context,
        0,
        b"IMAP-fake-IDLEing...\x00" as *const u8 as *const libc::c_char,
    );
    let mut do_fake_idle: libc::c_int = 1;
    while 0 != do_fake_idle {
        let seconds_to_wait = if fake_idle_start_time.elapsed().unwrap() < Duration::new(3 * 60, 0)
        {
            Duration::new(5, 0)
        } else {
            Duration::new(60, 0)
        };

        let &(ref lock, ref cvar) = &*imap.watch.clone();

        let mut watch = lock.lock().unwrap();

        loop {
            let res = cvar.wait_timeout(watch, seconds_to_wait).unwrap();
            watch = res.0;
            if *watch {
                do_fake_idle = 0;
            }
            if *watch || res.1.timed_out() {
                break;
            }
        }

        *watch = false;
        if do_fake_idle == 0 {
            return;
        }
        if 0 != setup_handle_if_needed(context, imap) {
            if 0 != fetch_from_single_folder(context, imap, imap.watch_folder) {
                do_fake_idle = 0;
            }
        }
    }
}

pub unsafe fn dc_imap_interrupt_idle(imap: &mut dc_imap_t) {
    if 0 != imap.can_idle {
        if !imap.etpan.is_null() && !(*imap.etpan).imap_stream.is_null() {
            mailstream_interrupt_idle((*imap.etpan).imap_stream);
        }
    }

    let &(ref lock, ref cvar) = &*imap.watch.clone();
    let mut watch = lock.lock().unwrap();

    *watch = true;
    cvar.notify_one();
}

pub unsafe fn dc_imap_move(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
    uid: uint32_t,
    dest_folder: *const libc::c_char,
    dest_uid: *mut uint32_t,
) -> dc_imap_res {
    let mut current_block: u64;
    let mut res: dc_imap_res = DC_RETRY_LATER;
    let mut r: libc::c_int = 0;
    let mut set: *mut mailimap_set = mailimap_set_new_single(uid);
    let mut res_uid: uint32_t = 0 as uint32_t;
    let mut res_setsrc: *mut mailimap_set = 0 as *mut mailimap_set;
    let mut res_setdest: *mut mailimap_set = 0 as *mut mailimap_set;
    if folder.is_null()
        || uid == 0 as libc::c_uint
        || dest_folder.is_null()
        || dest_uid.is_null()
        || set.is_null()
    {
        res = DC_FAILED
    } else if strcasecmp(folder, dest_folder) == 0 {
        dc_log_info(
            context,
            0,
            b"Skip moving message; message %s/%i is already in %s...\x00" as *const u8
                as *const libc::c_char,
            folder,
            uid as libc::c_int,
            dest_folder,
        );
        res = DC_ALREADY_DONE
    } else {
        dc_log_info(
            context,
            0,
            b"Moving message %s/%i to %s...\x00" as *const u8 as *const libc::c_char,
            folder,
            uid as libc::c_int,
            dest_folder,
        );
        if select_folder(context, imap, folder) == 0 {
            dc_log_warning(
                context,
                0,
                b"Cannot select folder %s for moving message.\x00" as *const u8
                    as *const libc::c_char,
                folder,
            );
        } else {
            r = mailimap_uidplus_uid_move(
                imap.etpan,
                set,
                dest_folder,
                &mut res_uid,
                &mut res_setsrc,
                &mut res_setdest,
            );
            if 0 != dc_imap_is_error(context, imap, r) {
                if !res_setsrc.is_null() {
                    mailimap_set_free(res_setsrc);
                    res_setsrc = 0 as *mut mailimap_set
                }
                if !res_setdest.is_null() {
                    mailimap_set_free(res_setdest);
                    res_setdest = 0 as *mut mailimap_set
                }
                dc_log_info(
                    context,
                    0,
                    b"Cannot move message, fallback to COPY/DELETE %s/%i to %s...\x00" as *const u8
                        as *const libc::c_char,
                    folder,
                    uid as libc::c_int,
                    dest_folder,
                );
                r = mailimap_uidplus_uid_copy(
                    imap.etpan,
                    set,
                    dest_folder,
                    &mut res_uid,
                    &mut res_setsrc,
                    &mut res_setdest,
                );
                if 0 != dc_imap_is_error(context, imap, r) {
                    dc_log_info(
                        context,
                        0,
                        b"Cannot copy message.\x00" as *const u8 as *const libc::c_char,
                    );
                    current_block = 14415637129417834392;
                } else {
                    if add_flag(imap, uid, mailimap_flag_new_deleted()) == 0 {
                        dc_log_warning(
                            context,
                            0,
                            b"Cannot mark message as \"Deleted\".\x00" as *const u8
                                as *const libc::c_char,
                        );
                    }
                    imap.selected_folder_needs_expunge = 1;
                    current_block = 1538046216550696469;
                }
            } else {
                current_block = 1538046216550696469;
            }
            match current_block {
                14415637129417834392 => {}
                _ => {
                    if !res_setdest.is_null() {
                        let mut cur: *mut clistiter = (*(*res_setdest).set_list).first;
                        if !cur.is_null() {
                            let mut item: *mut mailimap_set_item = 0 as *mut mailimap_set_item;
                            item = (if !cur.is_null() {
                                (*cur).data
                            } else {
                                0 as *mut libc::c_void
                            }) as *mut mailimap_set_item;
                            *dest_uid = (*item).set_first
                        }
                    }
                    res = DC_SUCCESS
                }
            }
        }
    }
    if !set.is_null() {
        mailimap_set_free(set);
        set = 0 as *mut mailimap_set
    }
    if !res_setsrc.is_null() {
        mailimap_set_free(res_setsrc);
        res_setsrc = 0 as *mut mailimap_set
    }
    if !res_setdest.is_null() {
        mailimap_set_free(res_setdest);
        res_setdest = 0 as *mut mailimap_set
    }
    return (if res as libc::c_uint == DC_RETRY_LATER as libc::c_int as libc::c_uint {
        (if 0 != imap.should_reconnect {
            DC_RETRY_LATER as libc::c_int
        } else {
            DC_FAILED as libc::c_int
        }) as libc::c_uint
    } else {
        res as libc::c_uint
    }) as dc_imap_res;
}

unsafe fn add_flag(
    imap: &mut dc_imap_t,
    server_uid: uint32_t,
    flag: *mut mailimap_flag,
) -> libc::c_int {
    let mut flag_list: *mut mailimap_flag_list = 0 as *mut mailimap_flag_list;
    let mut store_att_flags: *mut mailimap_store_att_flags = 0 as *mut mailimap_store_att_flags;
    let mut set: *mut mailimap_set = mailimap_set_new_single(server_uid);
    if !(imap.etpan.is_null()) {
        flag_list = mailimap_flag_list_new_empty();
        mailimap_flag_list_add(flag_list, flag);
        store_att_flags = mailimap_store_att_flags_new_add_flags(flag_list);
        mailimap_uid_store(imap.etpan, set, store_att_flags);
    }
    if !store_att_flags.is_null() {
        mailimap_store_att_flags_free(store_att_flags);
    }
    if !set.is_null() {
        mailimap_set_free(set);
        set = 0 as *mut mailimap_set
    }
    if 0 != imap.should_reconnect {
        0
    } else {
        1
    }
}

pub unsafe fn dc_imap_set_seen(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
    uid: uint32_t,
) -> dc_imap_res {
    let mut res: dc_imap_res = DC_RETRY_LATER;
    if folder.is_null() || uid == 0 as libc::c_uint {
        res = DC_FAILED
    } else if !imap.etpan.is_null() {
        dc_log_info(
            context,
            0,
            b"Marking message %s/%i as seen...\x00" as *const u8 as *const libc::c_char,
            folder,
            uid as libc::c_int,
        );
        if select_folder(context, imap, folder) == 0 {
            dc_log_warning(
                context,
                0,
                b"Cannot select folder %s for setting SEEN flag.\x00" as *const u8
                    as *const libc::c_char,
                folder,
            );
        } else if add_flag(imap, uid, mailimap_flag_new_seen()) == 0 {
            dc_log_warning(
                context,
                0,
                b"Cannot mark message as seen.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            res = DC_SUCCESS
        }
    }
    return (if res as libc::c_uint == DC_RETRY_LATER as libc::c_int as libc::c_uint {
        (if 0 != imap.should_reconnect {
            DC_RETRY_LATER as libc::c_int
        } else {
            DC_FAILED as libc::c_int
        }) as libc::c_uint
    } else {
        res as libc::c_uint
    }) as dc_imap_res;
}

pub unsafe fn dc_imap_set_mdnsent(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    folder: *const libc::c_char,
    uid: uint32_t,
) -> dc_imap_res {
    let mut can_create_flag: libc::c_int = 0;
    let mut current_block: u64;
    // returns 0=job should be retried later, 1=job done, 2=job done and flag just set
    let mut res: dc_imap_res = DC_RETRY_LATER;
    let mut set: *mut mailimap_set = mailimap_set_new_single(uid);
    let mut fetch_result: *mut clist = 0 as *mut clist;
    if folder.is_null() || uid == 0 as libc::c_uint || set.is_null() {
        res = DC_FAILED
    } else if !imap.etpan.is_null() {
        dc_log_info(
            context,
            0,
            b"Marking message %s/%i as $MDNSent...\x00" as *const u8 as *const libc::c_char,
            folder,
            uid as libc::c_int,
        );
        if select_folder(context, imap, folder) == 0 {
            dc_log_warning(
                context,
                0,
                b"Cannot select folder %s for setting $MDNSent flag.\x00" as *const u8
                    as *const libc::c_char,
                folder,
            );
        } else {
            /* Check if the folder can handle the `$MDNSent` flag (see RFC 3503).  If so, and not set: set the flags and return this information.
            If the folder cannot handle the `$MDNSent` flag, we risk duplicated MDNs; it's up to the receiving MUA to handle this then (eg. Delta Chat has no problem with this). */
            can_create_flag = 0;
            if !(*imap.etpan).imap_selection_info.is_null()
                && !(*(*imap.etpan).imap_selection_info)
                    .sel_perm_flags
                    .is_null()
            {
                let mut iter: *mut clistiter = 0 as *mut clistiter;
                iter = (*(*(*imap.etpan).imap_selection_info).sel_perm_flags).first;
                while !iter.is_null() {
                    let mut fp: *mut mailimap_flag_perm = (if !iter.is_null() {
                        (*iter).data
                    } else {
                        0 as *mut libc::c_void
                    })
                        as *mut mailimap_flag_perm;
                    if !fp.is_null() {
                        if (*fp).fl_type == MAILIMAP_FLAG_PERM_ALL as libc::c_int {
                            can_create_flag = 1;
                            break;
                        } else if (*fp).fl_type == MAILIMAP_FLAG_PERM_FLAG as libc::c_int
                            && !(*fp).fl_flag.is_null()
                        {
                            let mut fl: *mut mailimap_flag = (*fp).fl_flag as *mut mailimap_flag;
                            if (*fl).fl_type == MAILIMAP_FLAG_KEYWORD as libc::c_int
                                && !(*fl).fl_data.fl_keyword.is_null()
                                && strcmp(
                                    (*fl).fl_data.fl_keyword,
                                    b"$MDNSent\x00" as *const u8 as *const libc::c_char,
                                ) == 0
                            {
                                can_create_flag = 1;
                                break;
                            }
                        }
                    }
                    iter = if !iter.is_null() {
                        (*iter).next
                    } else {
                        0 as *mut clistcell_s
                    }
                }
            }
            if 0 != can_create_flag {
                let mut r: libc::c_int =
                    mailimap_uid_fetch(imap.etpan, set, imap.fetch_type_flags, &mut fetch_result);
                if 0 != dc_imap_is_error(context, imap, r) || fetch_result.is_null() {
                    fetch_result = 0 as *mut clist
                } else {
                    let mut cur: *mut clistiter = (*fetch_result).first;
                    if !cur.is_null() {
                        if 0 != peek_flag_keyword(
                            (if !cur.is_null() {
                                (*cur).data
                            } else {
                                0 as *mut libc::c_void
                            }) as *mut mailimap_msg_att,
                            b"$MDNSent\x00" as *const u8 as *const libc::c_char,
                        ) {
                            res = DC_ALREADY_DONE;
                            current_block = 14832935472441733737;
                        } else if add_flag(
                            imap,
                            uid,
                            mailimap_flag_new_flag_keyword(dc_strdup(
                                b"$MDNSent\x00" as *const u8 as *const libc::c_char,
                            )),
                        ) == 0
                        {
                            current_block = 17044610252497760460;
                        } else {
                            res = DC_SUCCESS;
                            current_block = 14832935472441733737;
                        }
                        match current_block {
                            17044610252497760460 => {}
                            _ => {
                                dc_log_info(
                                    context,
                                    0,
                                    if res as libc::c_uint
                                        == DC_SUCCESS as libc::c_int as libc::c_uint
                                    {
                                        b"$MDNSent just set and MDN will be sent.\x00" as *const u8
                                            as *const libc::c_char
                                    } else {
                                        b"$MDNSent already set and MDN already sent.\x00"
                                            as *const u8
                                            as *const libc::c_char
                                    },
                                );
                            }
                        }
                    }
                }
            } else {
                res = DC_SUCCESS;
                dc_log_info(
                    context,
                    0,
                    b"Cannot store $MDNSent flags, risk sending duplicate MDN.\x00" as *const u8
                        as *const libc::c_char,
                );
            }
        }
    }
    if !set.is_null() {
        mailimap_set_free(set);
        set = 0 as *mut mailimap_set
    }
    if !fetch_result.is_null() {
        mailimap_fetch_list_free(fetch_result);
        fetch_result = 0 as *mut clist
    }

    (if res as libc::c_uint == DC_RETRY_LATER as libc::c_int as libc::c_uint {
        (if 0 != imap.should_reconnect {
            DC_RETRY_LATER as libc::c_int
        } else {
            DC_FAILED as libc::c_int
        }) as libc::c_uint
    } else {
        res as libc::c_uint
    }) as dc_imap_res
}

unsafe fn peek_flag_keyword(
    msg_att: *mut mailimap_msg_att,
    flag_keyword: *const libc::c_char,
) -> libc::c_int {
    if msg_att.is_null() || (*msg_att).att_list.is_null() || flag_keyword.is_null() {
        return 0;
    }
    let mut iter1: *mut clistiter = 0 as *mut clistiter;
    let mut iter2: *mut clistiter = 0 as *mut clistiter;
    iter1 = (*(*msg_att).att_list).first;
    while !iter1.is_null() {
        let mut item: *mut mailimap_msg_att_item = (if !iter1.is_null() {
            (*iter1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimap_msg_att_item;
        if !item.is_null() {
            if (*item).att_type == MAILIMAP_MSG_ATT_ITEM_DYNAMIC as libc::c_int {
                if !(*(*item).att_data.att_dyn).att_list.is_null() {
                    iter2 = (*(*(*item).att_data.att_dyn).att_list).first;
                    while !iter2.is_null() {
                        let mut flag_fetch: *mut mailimap_flag_fetch = (if !iter2.is_null() {
                            (*iter2).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *mut mailimap_flag_fetch;
                        if !flag_fetch.is_null()
                            && (*flag_fetch).fl_type == MAILIMAP_FLAG_FETCH_OTHER as libc::c_int
                        {
                            let mut flag: *mut mailimap_flag = (*flag_fetch).fl_flag;
                            if !flag.is_null() {
                                if (*flag).fl_type == MAILIMAP_FLAG_KEYWORD as libc::c_int
                                    && !(*flag).fl_data.fl_keyword.is_null()
                                    && strcmp((*flag).fl_data.fl_keyword, flag_keyword) == 0
                                {
                                    return 1;
                                }
                            }
                        }
                        iter2 = if !iter2.is_null() {
                            (*iter2).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                }
            }
        }
        iter1 = if !iter1.is_null() {
            (*iter1).next
        } else {
            0 as *mut clistcell_s
        }
    }
    0
}

/* only returns 0 on connection problems; we should try later again in this case */
pub unsafe fn dc_imap_delete_msg(
    context: &dc_context_t,
    imap: &mut dc_imap_t,
    rfc724_mid: *const libc::c_char,
    folder: *const libc::c_char,
    mut server_uid: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut fetch_result: *mut clist = 0 as *mut clist;
    let mut is_rfc724_mid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut new_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    if rfc724_mid.is_null()
        || folder.is_null()
        || *folder.offset(0isize) as libc::c_int == 0
        || server_uid == 0 as libc::c_uint
    {
        success = 1
    } else {
        dc_log_info(
            context,
            0,
            b"Marking message \"%s\", %s/%i for deletion...\x00" as *const u8
                as *const libc::c_char,
            rfc724_mid,
            folder,
            server_uid as libc::c_int,
        );
        if select_folder(context, imap, folder) == 0 {
            dc_log_warning(
                context,
                0,
                b"Cannot select folder %s for deleting message.\x00" as *const u8
                    as *const libc::c_char,
                folder,
            );
        } else {
            let mut cur: *mut clistiter = 0 as *mut clistiter;
            let mut is_quoted_rfc724_mid: *const libc::c_char = 0 as *const libc::c_char;
            let mut set: *mut mailimap_set = mailimap_set_new_single(server_uid);
            r = mailimap_uid_fetch(imap.etpan, set, imap.fetch_type_prefetch, &mut fetch_result);
            if !set.is_null() {
                mailimap_set_free(set);
                set = 0 as *mut mailimap_set
            }
            if 0 != dc_imap_is_error(context, imap, r) || fetch_result.is_null() {
                fetch_result = 0 as *mut clist;
                dc_log_warning(
                    context,
                    0,
                    b"Cannot delete on IMAP, %s/%i not found.\x00" as *const u8
                        as *const libc::c_char,
                    folder,
                    server_uid as libc::c_int,
                );
                server_uid = 0 as uint32_t
            }
            cur = (*fetch_result).first;
            if cur.is_null()
                || {
                    is_quoted_rfc724_mid = peek_rfc724_mid(
                        (if !cur.is_null() {
                            (*cur).data
                        } else {
                            0 as *mut libc::c_void
                        }) as *mut mailimap_msg_att,
                    );
                    is_quoted_rfc724_mid.is_null()
                }
                || {
                    is_rfc724_mid = unquote_rfc724_mid(is_quoted_rfc724_mid);
                    is_rfc724_mid.is_null()
                }
                || strcmp(is_rfc724_mid, rfc724_mid) != 0
            {
                dc_log_warning(
                    context,
                    0,
                    b"Cannot delete on IMAP, %s/%i does not match %s.\x00" as *const u8
                        as *const libc::c_char,
                    folder,
                    server_uid as libc::c_int,
                    rfc724_mid,
                );
                server_uid = 0 as uint32_t
            }
            /* mark the message for deletion */
            if add_flag(imap, server_uid, mailimap_flag_new_deleted()) == 0 {
                dc_log_warning(
                    context,
                    0,
                    b"Cannot mark message as \"Deleted\".\x00" as *const u8 as *const libc::c_char,
                );
            } else {
                imap.selected_folder_needs_expunge = 1;
                success = 1
            }
        }
    }
    if !fetch_result.is_null() {
        mailimap_fetch_list_free(fetch_result);
        fetch_result = 0 as *mut clist
    }
    free(is_rfc724_mid as *mut libc::c_void);
    free(new_folder as *mut libc::c_void);

    if 0 != success {
        1
    } else {
        dc_imap_is_connected(imap)
    }
}
