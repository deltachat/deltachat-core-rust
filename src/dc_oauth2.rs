use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_context::dc_context_t;
use crate::dc_imap::dc_imap_t;
use crate::dc_lot::dc_lot_t;
use crate::dc_smtp::dc_smtp_t;
use crate::dc_sqlite3::dc_sqlite3_t;
use crate::types::*;

extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn atol(_: *const libc::c_char) -> libc::c_long;
    #[no_mangle]
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strndup(_: *const libc::c_char, _: libc::c_ulong) -> *mut libc::c_char;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn pthread_mutex_lock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_unlock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn dc_urlencode(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_str_replace(
        haystack: *mut *mut libc::c_char,
        needle: *const libc::c_char,
        replacement: *const libc::c_char,
    ) -> libc::c_int;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    /* handle configurations, private */
    #[no_mangle]
    fn dc_sqlite3_set_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        value: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_addr_normalize(addr: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_sqlite3_set_config_int64(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        value: int64_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_sqlite3_get_config_int64(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: int64_t,
    ) -> int64_t;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    /* *
     * Run JSON parser. It parses a JSON data string into and array of tokens, each describing
     * a single JSON object.
     */
    #[no_mangle]
    fn jsmn_parse(
        parser: *mut jsmn_parser,
        js: *const libc::c_char,
        len: size_t,
        tokens: *mut jsmntok_t,
        num_tokens: libc::c_uint,
    ) -> libc::c_int;
    /* *
     * Create JSON parser over an array of tokens
     */
    #[no_mangle]
    fn jsmn_init(parser: *mut jsmn_parser);
}

/* define DC_USE_RPGP to enable use of rPGP instead of netpgp where available;
preferrably, this should be done in the project configuration currently */
//#define DC_USE_RPGP 1
/* Includes that are used frequently.  This file may also be used to create predefined headers. */

/* *
 * Callback function that should be given to dc_context_new().
 *
 * @memberof dc_context_t
 * @param context The context object as returned by dc_context_new().
 * @param event one of the @ref DC_EVENT constants
 * @param data1 depends on the event parameter
 * @param data2 depends on the event parameter
 * @return return 0 unless stated otherwise in the event parameter documentation
 */
pub type dc_callback_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_context_t,
        _: libc::c_int,
        _: uintptr_t,
        _: uintptr_t,
    ) -> uintptr_t,
>;
/* *
 * @mainpage Getting started
 *
 * This document describes how to handle the Delta Chat core library.
 * For general information about Delta Chat itself,
 * see <https://delta.chat> and <https://github.com/deltachat>.
 *
 * Let's start.
 *
 * First of all, you have to **define an event-handler-function**
 * that is called by the library on specific events
 * (eg. when the configuration is done or when fresh messages arrive).
 * With this function you can create a Delta Chat context then:
 *
 * ~~~
 * #include <deltachat.h>
 *
 * uintptr_t event_handler_func(dc_context_t* context, int event,
 *                              uintptr_t data1, uintptr_t data2)
 * {
 *     return 0; // for unhandled events, it is always safe to return 0
 * }
 *
 * dc_context_t* context = dc_context_new(event_handler_func, NULL, NULL);
 * ~~~
 *
 * After that, you should make sure,
 * sending and receiving jobs are processed as needed.
 * For this purpose, you have to **create two threads:**
 *
 * ~~~
 * #include <pthread.h>
 *
 * void* imap_thread_func(void* context)
 * {
 *     while (true) {
 *         dc_perform_imap_jobs(context);
 *         dc_perform_imap_fetch(context);
 *         dc_perform_imap_idle(context);
 *     }
 * }
 *
 * void* smtp_thread_func(void* context)
 * {
 *     while (true) {
 *         dc_perform_smtp_jobs(context);
 *         dc_perform_smtp_idle(context);
 *     }
 * }
 *
 * static pthread_t imap_thread, smtp_thread;
 * pthread_create(&imap_thread, NULL, imap_thread_func, context);
 * pthread_create(&smtp_thread, NULL, smtp_thread_func, context);
 * ~~~
 *
 * The example above uses "pthreads",
 * however, you can also use anything else for thread handling.
 * NB: The deltachat-core library itself does not create any threads on its own,
 * however, functions, unless stated otherwise, are thread-safe.
 *
 * After that you can  **define and open a database.**
 * The database is a normal sqlite-file and is created as needed:
 *
 * ~~~
 * dc_open(context, "example.db", NULL);
 * ~~~
 *
 * Now you can **configure the context:**
 *
 * ~~~
 * // use some real test credentials here
 * dc_set_config(context, "addr", "alice@example.org");
 * dc_set_config(context, "mail_pw", "***");
 * dc_configure(context);
 * ~~~
 *
 * dc_configure() returns immediately, the configuration itself may take a while
 * and is done by a job in the imap-thread you've defined above.
 * Once done, the #DC_EVENT_CONFIGURE_PROGRESS reports success
 * to the event_handler_func() that is also defined above.
 *
 * The configuration result is saved in the database,
 * on subsequent starts it is not needed to call dc_configure()
 * (you can check this using dc_is_configured()).
 *
 * Now you can **send the first message:**
 *
 * ~~~
 * // use a real testing address here
 * uint32_t contact_id = dc_create_contact(context, NULL, "bob@example.org");
 * uint32_t chat_id    = dc_create_chat_by_contact_id(context, contact_id);
 *
 * dc_send_text_msg(context, chat_id, "Hi, here is my first message!");
 * ~~~
 *
 * dc_send_text_msg() returns immediately;
 * the sending itself is done by a job in the smtp-thread you've defined above.
 * If you check the testing address (bob)
 * and you should have received a normal email.
 * Answer this email in any email program with "Got it!"
 * and the imap-thread you've create above will **receive the message**.
 *
 * You can then **list all messages** of a chat as follow:
 *
 * ~~~
 * dc_array_t* msglist = dc_get_chat_msgs(context, chat_id, 0, 0);
 * for (int i = 0; i < dc_array_get_cnt(msglist); i++)
 * {
 *     uint32_t  msg_id = dc_array_get_id(msglist, i);
 *     dc_msg_t* msg    = dc_get_msg(context, msg_id);
 *     char*     text   = dc_msg_get_text(msg);
 *
 *     printf("Message %i: %s\n", i+1, text);
 *
 *     free(text);
 *     dc_msg_unref(msg);
 * }
 * dc_array_unref(msglist);
 * ~~~
 *
 * This will output the following two lines:
 *
 * ~~~
 * Message 1: Hi, here is my first message!
 * Message 2: Got it!
 * ~~~
 *
 *
 * ## Class reference
 *
 * For a class reference, see the "Classes" link atop.
 *
 *
 * ## Further hints
 *
 * Here are some additional, unsorted hints that may be useful.
 *
 * - For `get`-functions, you have to unref the return value in some way.
 *
 * - Strings in function arguments or return values are usually UTF-8 encoded.
 *
 * - The issue-tracker for the core library is here:
 *   <https://github.com/deltachat/deltachat-core/issues>
 *
 * The following points are important mainly
 * for the authors of the library itself:
 *
 * - For indentation, use tabs.
 *   Alignments that are not placed at the beginning of a line
 *   should be done with spaces.
 *
 * - For padding between functions,
 *   classes etc. use 2 empty lines
 *
 * - Source files are encoded as UTF-8 with Unix line endings
 *   (a simple `LF`, `0x0A` or `\n`)
 *
 * If you need further assistance,
 * please do not hesitate to contact us
 * through the channels shown at https://delta.chat/en/contribute
 *
 * Please keep in mind, that your derived work
 * must respect the Mozilla Public License 2.0 of libdeltachat
 * and the respective licenses of the libraries libdeltachat links with.
 *
 * See you.
 */

pub type dc_receive_imf_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_imap_t,
        _: *const libc::c_char,
        _: size_t,
        _: *const libc::c_char,
        _: uint32_t,
        _: uint32_t,
    ) -> (),
>;

pub type dc_precheck_imf_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_imap_t,
        _: *const libc::c_char,
        _: *const libc::c_char,
        _: uint32_t,
    ) -> libc::c_int,
>;
pub type dc_set_config_t = Option<
    unsafe extern "C" fn(_: *mut dc_imap_t, _: *const libc::c_char, _: *const libc::c_char) -> (),
>;
pub type dc_get_config_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_imap_t,
        _: *const libc::c_char,
        _: *const libc::c_char,
    ) -> *mut libc::c_char,
>;
/* ** library-private **********************************************************/

/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_sqlite3 {
    pub cobj: *mut sqlite3,
    pub context: *mut dc_context_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct oauth2_t {
    pub client_id: *mut libc::c_char,
    pub get_code: *mut libc::c_char,
    pub init_token: *mut libc::c_char,
    pub refresh_token: *mut libc::c_char,
    pub get_userinfo: *mut libc::c_char,
}
/* *
 * JSON token description.
 * type		type (object, array, string etc.)
 * start	start position in JSON data string
 * end		end position in JSON data string
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmntok_t {
    pub type_0: jsmntype_t,
    pub start: libc::c_int,
    pub end: libc::c_int,
    pub size: libc::c_int,
}
/*
Copyright (c) 2010 Serge A. Zaitsev

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
*/
/* *
 * JSON type identifier. Basic types are:
 * 	o Object
 * 	o Array
 * 	o String
 * 	o Other primitive: number, boolean (true/false) or null
 */
pub type jsmntype_t = libc::c_uint;
pub const JSMN_PRIMITIVE: jsmntype_t = 4;
pub const JSMN_STRING: jsmntype_t = 3;
pub const JSMN_ARRAY: jsmntype_t = 2;
pub const JSMN_OBJECT: jsmntype_t = 1;
pub const JSMN_UNDEFINED: jsmntype_t = 0;
/* *
 * JSON parser. Contains an array of token blocks available. Also stores
 * the string being parsed now and current position in that string
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmn_parser {
    pub pos: libc::c_uint,
    pub toknext: libc::c_uint,
    pub toksuper: libc::c_int,
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_oauth2_url(
    mut context: *mut dc_context_t,
    mut addr: *const libc::c_char,
    mut redirect_uri: *const libc::c_char,
) -> *mut libc::c_char {
    let mut oauth2: *mut oauth2_t = 0 as *mut oauth2_t;
    let mut oauth2_url: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || redirect_uri.is_null()
        || *redirect_uri.offset(0isize) as libc::c_int == 0i32)
    {
        oauth2 = get_info(addr);
        if !oauth2.is_null() {
            dc_sqlite3_set_config(
                (*context).sql,
                b"oauth2_pending_redirect_uri\x00" as *const u8 as *const libc::c_char,
                redirect_uri,
            );
            oauth2_url = dc_strdup((*oauth2).get_code);
            replace_in_uri(
                &mut oauth2_url,
                b"$CLIENT_ID\x00" as *const u8 as *const libc::c_char,
                (*oauth2).client_id,
            );
            replace_in_uri(
                &mut oauth2_url,
                b"$REDIRECT_URI\x00" as *const u8 as *const libc::c_char,
                redirect_uri,
            );
        }
    }
    free(oauth2 as *mut libc::c_void);
    return oauth2_url;
}
unsafe extern "C" fn replace_in_uri(
    mut uri: *mut *mut libc::c_char,
    mut key: *const libc::c_char,
    mut value: *const libc::c_char,
) {
    if !uri.is_null() && !key.is_null() && !value.is_null() {
        let mut value_urlencoded: *mut libc::c_char = dc_urlencode(value);
        dc_str_replace(uri, key, value_urlencoded);
        free(value_urlencoded as *mut libc::c_void);
    };
}
unsafe extern "C" fn get_info(mut addr: *const libc::c_char) -> *mut oauth2_t {
    let mut oauth2: *mut oauth2_t = 0 as *mut oauth2_t;
    let mut addr_normalized: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut domain: *const libc::c_char = 0 as *const libc::c_char;
    addr_normalized = dc_addr_normalize(addr);
    domain = strchr(addr_normalized, '@' as i32);
    if !(domain.is_null() || *domain.offset(0isize) as libc::c_int == 0i32) {
        domain = domain.offset(1isize);
        if strcasecmp(domain, b"gmail.com\x00" as *const u8 as *const libc::c_char) == 0i32
            || strcasecmp(
                domain,
                b"googlemail.com\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
        {
            oauth2 = calloc(
                1i32 as libc::c_ulong,
                ::std::mem::size_of::<oauth2_t>() as libc::c_ulong,
            ) as *mut oauth2_t;
            (*oauth2).client_id =
                b"959970109878-4mvtgf6feshskf7695nfln6002mom908.apps.googleusercontent.com\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char;
            (*oauth2).get_code =
                b"https://accounts.google.com/o/oauth2/auth?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F%20email&access_type=offline\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char;
            (*oauth2).init_token =
                b"https://accounts.google.com/o/oauth2/token?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&code=$CODE&grant_type=authorization_code\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char;
            (*oauth2).refresh_token =
                b"https://accounts.google.com/o/oauth2/token?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&refresh_token=$REFRESH_TOKEN&grant_type=refresh_token\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char;
            (*oauth2).get_userinfo =
                b"https://www.googleapis.com/oauth2/v1/userinfo?alt=json&access_token=$ACCESS_TOKEN\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char
        } else if strcasecmp(
            domain,
            b"yandex.com\x00" as *const u8 as *const libc::c_char,
        ) == 0i32
            || strcasecmp(domain, b"yandex.ru\x00" as *const u8 as *const libc::c_char) == 0i32
            || strcasecmp(domain, b"yandex.ua\x00" as *const u8 as *const libc::c_char) == 0i32
        {
            oauth2 = calloc(
                1i32 as libc::c_ulong,
                ::std::mem::size_of::<oauth2_t>() as libc::c_ulong,
            ) as *mut oauth2_t;
            (*oauth2).client_id = b"c4d0b6735fc8420a816d7e1303469341\x00" as *const u8
                as *const libc::c_char as *mut libc::c_char;
            (*oauth2).get_code =
                b"https://oauth.yandex.com/authorize?client_id=$CLIENT_ID&response_type=code&scope=mail%3Aimap_full%20mail%3Asmtp&force_confirm=true\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char;
            (*oauth2).init_token =
                b"https://oauth.yandex.com/token?grant_type=authorization_code&code=$CODE&client_id=$CLIENT_ID&client_secret=58b8c6e94cf44fbe952da8511955dacf\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char;
            (*oauth2).refresh_token =
                b"https://oauth.yandex.com/token?grant_type=refresh_token&refresh_token=$REFRESH_TOKEN&client_id=$CLIENT_ID&client_secret=58b8c6e94cf44fbe952da8511955dacf\x00"
                    as *const u8 as *const libc::c_char as *mut libc::c_char
        }
    }
    free(addr_normalized as *mut libc::c_void);
    return oauth2;
}
// the following function may block due http-requests;
// must not be called from the main thread or by the ui!
#[no_mangle]
pub unsafe extern "C" fn dc_get_oauth2_access_token(
    mut context: *mut dc_context_t,
    mut addr: *const libc::c_char,
    mut code: *const libc::c_char,
    mut flags: libc::c_int,
) -> *mut libc::c_char {
    let mut current_block: u64;
    let mut oauth2: *mut oauth2_t = 0 as *mut oauth2_t;
    let mut access_token: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut refresh_token: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut refresh_token_for: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut redirect_uri: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut update_redirect_uri_on_success: libc::c_int = 0i32;
    let mut token_url: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut expires_in: time_t = 0i32 as time_t;
    let mut error: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut error_description: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut json: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parser: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    // we do not expect nor read more tokens
    let mut tok: [jsmntok_t; 128] = [jsmntok_t {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 128];
    let mut tok_cnt: libc::c_int = 0i32;
    let mut locked: libc::c_int = 0i32;
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || code.is_null()
        || *code.offset(0isize) as libc::c_int == 0i32
    {
        dc_log_warning(
            context,
            0i32,
            b"Internal OAuth2 error\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        oauth2 = get_info(addr);
        if oauth2.is_null() {
            dc_log_warning(
                context,
                0i32,
                b"Internal OAuth2 error: 2\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            pthread_mutex_lock(&mut (*context).oauth2_critical);
            locked = 1i32;
            // read generated token
            if 0 == flags & 0x1i32 && 0 == is_expired(context) {
                access_token = dc_sqlite3_get_config(
                    (*context).sql,
                    b"oauth2_access_token\x00" as *const u8 as *const libc::c_char,
                    0 as *const libc::c_char,
                );
                if !access_token.is_null() {
                    // success
                    current_block = 16914036240511706173;
                } else {
                    current_block = 2838571290723028321;
                }
            } else {
                current_block = 2838571290723028321;
            }
            match current_block {
                16914036240511706173 => {}
                _ => {
                    refresh_token = dc_sqlite3_get_config(
                        (*context).sql,
                        b"oauth2_refresh_token\x00" as *const u8 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                    refresh_token_for = dc_sqlite3_get_config(
                        (*context).sql,
                        b"oauth2_refresh_token_for\x00" as *const u8 as *const libc::c_char,
                        b"unset\x00" as *const u8 as *const libc::c_char,
                    );
                    if refresh_token.is_null() || strcmp(refresh_token_for, code) != 0i32 {
                        dc_log_info(
                            context,
                            0i32,
                            b"Generate OAuth2 refresh_token and access_token...\x00" as *const u8
                                as *const libc::c_char,
                        );
                        redirect_uri = dc_sqlite3_get_config(
                            (*context).sql,
                            b"oauth2_pending_redirect_uri\x00" as *const u8 as *const libc::c_char,
                            b"unset\x00" as *const u8 as *const libc::c_char,
                        );
                        update_redirect_uri_on_success = 1i32;
                        token_url = dc_strdup((*oauth2).init_token)
                    } else {
                        dc_log_info(
                            context,
                            0i32,
                            b"Regenerate OAuth2 access_token by refresh_token...\x00" as *const u8
                                as *const libc::c_char,
                        );
                        redirect_uri = dc_sqlite3_get_config(
                            (*context).sql,
                            b"oauth2_redirect_uri\x00" as *const u8 as *const libc::c_char,
                            b"unset\x00" as *const u8 as *const libc::c_char,
                        );
                        token_url = dc_strdup((*oauth2).refresh_token)
                    }
                    replace_in_uri(
                        &mut token_url,
                        b"$CLIENT_ID\x00" as *const u8 as *const libc::c_char,
                        (*oauth2).client_id,
                    );
                    replace_in_uri(
                        &mut token_url,
                        b"$REDIRECT_URI\x00" as *const u8 as *const libc::c_char,
                        redirect_uri,
                    );
                    replace_in_uri(
                        &mut token_url,
                        b"$CODE\x00" as *const u8 as *const libc::c_char,
                        code,
                    );
                    replace_in_uri(
                        &mut token_url,
                        b"$REFRESH_TOKEN\x00" as *const u8 as *const libc::c_char,
                        refresh_token,
                    );
                    json = (*context).cb.expect("non-null function pointer")(
                        context,
                        2110i32,
                        token_url as uintptr_t,
                        0i32 as uintptr_t,
                    ) as *mut libc::c_char;
                    if json.is_null() {
                        dc_log_warning(
                            context,
                            0i32,
                            b"Error calling OAuth2 at %s\x00" as *const u8 as *const libc::c_char,
                            token_url,
                        );
                    } else {
                        jsmn_init(&mut parser);
                        tok_cnt = jsmn_parse(
                            &mut parser,
                            json,
                            strlen(json),
                            tok.as_mut_ptr(),
                            (::std::mem::size_of::<[jsmntok_t; 128]>() as libc::c_ulong)
                                .wrapping_div(::std::mem::size_of::<jsmntok_t>() as libc::c_ulong)
                                as libc::c_uint,
                        );
                        if tok_cnt < 2i32
                            || tok[0usize].type_0 as libc::c_uint
                                != JSMN_OBJECT as libc::c_int as libc::c_uint
                        {
                            dc_log_warning(
                                context,
                                0i32,
                                b"Failed to parse OAuth2 json from %s\x00" as *const u8
                                    as *const libc::c_char,
                                token_url,
                            );
                        } else {
                            let mut i: libc::c_int = 1i32;
                            while i < tok_cnt {
                                if access_token.is_null()
                                    && jsoneq(
                                        json,
                                        &mut *tok.as_mut_ptr().offset(i as isize),
                                        b"access_token\x00" as *const u8 as *const libc::c_char,
                                    ) == 0i32
                                {
                                    access_token = jsondup(
                                        json,
                                        &mut *tok.as_mut_ptr().offset((i + 1i32) as isize),
                                    )
                                } else if refresh_token.is_null()
                                    && jsoneq(
                                        json,
                                        &mut *tok.as_mut_ptr().offset(i as isize),
                                        b"refresh_token\x00" as *const u8 as *const libc::c_char,
                                    ) == 0i32
                                {
                                    refresh_token = jsondup(
                                        json,
                                        &mut *tok.as_mut_ptr().offset((i + 1i32) as isize),
                                    )
                                } else if jsoneq(
                                    json,
                                    &mut *tok.as_mut_ptr().offset(i as isize),
                                    b"expires_in\x00" as *const u8 as *const libc::c_char,
                                ) == 0i32
                                {
                                    let mut expires_in_str: *mut libc::c_char = jsondup(
                                        json,
                                        &mut *tok.as_mut_ptr().offset((i + 1i32) as isize),
                                    );
                                    if !expires_in_str.is_null() {
                                        let mut val: time_t = atol(expires_in_str);
                                        if val > 20i32 as libc::c_long
                                            && val
                                                < (60i32 * 60i32 * 24i32 * 365i32 * 5i32)
                                                    as libc::c_long
                                        {
                                            expires_in = val
                                        }
                                        free(expires_in_str as *mut libc::c_void);
                                    }
                                } else if error.is_null()
                                    && jsoneq(
                                        json,
                                        &mut *tok.as_mut_ptr().offset(i as isize),
                                        b"error\x00" as *const u8 as *const libc::c_char,
                                    ) == 0i32
                                {
                                    error = jsondup(
                                        json,
                                        &mut *tok.as_mut_ptr().offset((i + 1i32) as isize),
                                    )
                                } else if error_description.is_null()
                                    && jsoneq(
                                        json,
                                        &mut *tok.as_mut_ptr().offset(i as isize),
                                        b"error_description\x00" as *const u8
                                            as *const libc::c_char,
                                    ) == 0i32
                                {
                                    error_description = jsondup(
                                        json,
                                        &mut *tok.as_mut_ptr().offset((i + 1i32) as isize),
                                    )
                                }
                                i += 1
                            }
                            if !error.is_null() || !error_description.is_null() {
                                dc_log_warning(
                                    context,
                                    0i32,
                                    b"OAuth error: %s: %s\x00" as *const u8 as *const libc::c_char,
                                    if !error.is_null() {
                                        error
                                    } else {
                                        b"unknown\x00" as *const u8 as *const libc::c_char
                                    },
                                    if !error_description.is_null() {
                                        error_description
                                    } else {
                                        b"no details\x00" as *const u8 as *const libc::c_char
                                    },
                                );
                            }
                            if !refresh_token.is_null()
                                && 0 != *refresh_token.offset(0isize) as libc::c_int
                            {
                                dc_sqlite3_set_config(
                                    (*context).sql,
                                    b"oauth2_refresh_token\x00" as *const u8 as *const libc::c_char,
                                    refresh_token,
                                );
                                dc_sqlite3_set_config(
                                    (*context).sql,
                                    b"oauth2_refresh_token_for\x00" as *const u8
                                        as *const libc::c_char,
                                    code,
                                );
                            }
                            // after that, save the access token.
                            // if it's unset, we may get it in the next round as we have the refresh_token now.
                            if access_token.is_null()
                                || *access_token.offset(0isize) as libc::c_int == 0i32
                            {
                                dc_log_warning(
                                    context,
                                    0i32,
                                    b"Failed to find OAuth2 access token\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                                dc_sqlite3_set_config(
                                    (*context).sql,
                                    b"oauth2_access_token\x00" as *const u8 as *const libc::c_char,
                                    access_token,
                                );
                                dc_sqlite3_set_config_int64(
                                    (*context).sql,
                                    b"oauth2_timestamp_expires\x00" as *const u8
                                        as *const libc::c_char,
                                    (if 0 != expires_in {
                                        time(0 as *mut time_t) + expires_in - 5i32 as libc::c_long
                                    } else {
                                        0i32 as libc::c_long
                                    }) as int64_t,
                                );
                                if 0 != update_redirect_uri_on_success {
                                    dc_sqlite3_set_config(
                                        (*context).sql,
                                        b"oauth2_redirect_uri\x00" as *const u8
                                            as *const libc::c_char,
                                        redirect_uri,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if 0 != locked {
        pthread_mutex_unlock(&mut (*context).oauth2_critical);
    }
    free(refresh_token as *mut libc::c_void);
    free(refresh_token_for as *mut libc::c_void);
    free(redirect_uri as *mut libc::c_void);
    free(token_url as *mut libc::c_void);
    free(json as *mut libc::c_void);
    free(error as *mut libc::c_void);
    free(error_description as *mut libc::c_void);
    free(oauth2 as *mut libc::c_void);
    return if !access_token.is_null() {
        access_token
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}
unsafe extern "C" fn jsondup(
    mut json: *const libc::c_char,
    mut tok: *mut jsmntok_t,
) -> *mut libc::c_char {
    if (*tok).type_0 as libc::c_uint == JSMN_STRING as libc::c_int as libc::c_uint
        || (*tok).type_0 as libc::c_uint == JSMN_PRIMITIVE as libc::c_int as libc::c_uint
    {
        return strndup(
            json.offset((*tok).start as isize),
            ((*tok).end - (*tok).start) as libc::c_ulong,
        );
    }
    return strdup(b"\x00" as *const u8 as *const libc::c_char);
}
unsafe extern "C" fn jsoneq(
    mut json: *const libc::c_char,
    mut tok: *mut jsmntok_t,
    mut s: *const libc::c_char,
) -> libc::c_int {
    if (*tok).type_0 as libc::c_uint == JSMN_STRING as libc::c_int as libc::c_uint
        && strlen(s) as libc::c_int == (*tok).end - (*tok).start
        && strncmp(
            json.offset((*tok).start as isize),
            s,
            ((*tok).end - (*tok).start) as libc::c_ulong,
        ) == 0i32
    {
        return 0i32;
    }
    return -1i32;
}
unsafe extern "C" fn is_expired(mut context: *mut dc_context_t) -> libc::c_int {
    let mut expire_timestamp: time_t = dc_sqlite3_get_config_int64(
        (*context).sql,
        b"oauth2_timestamp_expires\x00" as *const u8 as *const libc::c_char,
        0i32 as int64_t,
    ) as time_t;
    if expire_timestamp <= 0i32 as libc::c_long {
        return 0i32;
    }
    if expire_timestamp > time(0 as *mut time_t) {
        return 0i32;
    }
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_oauth2_addr(
    mut context: *mut dc_context_t,
    mut addr: *const libc::c_char,
    mut code: *const libc::c_char,
) -> *mut libc::c_char {
    let mut access_token: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut addr_out: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut oauth2: *mut oauth2_t = 0 as *mut oauth2_t;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || {
            oauth2 = get_info(addr);
            oauth2.is_null()
        }
        || (*oauth2).get_userinfo.is_null())
    {
        access_token = dc_get_oauth2_access_token(context, addr, code, 0i32);
        addr_out = get_oauth2_addr(context, oauth2, access_token);
        if addr_out.is_null() {
            free(access_token as *mut libc::c_void);
            access_token = dc_get_oauth2_access_token(context, addr, code, 0x1i32);
            addr_out = get_oauth2_addr(context, oauth2, access_token)
        }
    }
    free(access_token as *mut libc::c_void);
    free(oauth2 as *mut libc::c_void);
    return addr_out;
}
unsafe extern "C" fn get_oauth2_addr(
    mut context: *mut dc_context_t,
    mut oauth2: *const oauth2_t,
    mut access_token: *const libc::c_char,
) -> *mut libc::c_char {
    let mut addr_out: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut userinfo_url: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut json: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parser: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    // we do not expect nor read more tokens
    let mut tok: [jsmntok_t; 128] = [jsmntok_t {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 128];
    let mut tok_cnt: libc::c_int = 0i32;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || access_token.is_null()
        || *access_token.offset(0isize) as libc::c_int == 0i32
        || oauth2.is_null())
    {
        userinfo_url = dc_strdup((*oauth2).get_userinfo);
        replace_in_uri(
            &mut userinfo_url,
            b"$ACCESS_TOKEN\x00" as *const u8 as *const libc::c_char,
            access_token,
        );
        json = (*context).cb.expect("non-null function pointer")(
            context,
            2100i32,
            userinfo_url as uintptr_t,
            0i32 as uintptr_t,
        ) as *mut libc::c_char;
        if json.is_null() {
            dc_log_warning(
                context,
                0i32,
                b"Error getting userinfo.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            jsmn_init(&mut parser);
            tok_cnt = jsmn_parse(
                &mut parser,
                json,
                strlen(json),
                tok.as_mut_ptr(),
                (::std::mem::size_of::<[jsmntok_t; 128]>() as libc::c_ulong)
                    .wrapping_div(::std::mem::size_of::<jsmntok_t>() as libc::c_ulong)
                    as libc::c_uint,
            );
            if tok_cnt < 2i32
                || tok[0usize].type_0 as libc::c_uint != JSMN_OBJECT as libc::c_int as libc::c_uint
            {
                dc_log_warning(
                    context,
                    0i32,
                    b"Failed to parse userinfo.\x00" as *const u8 as *const libc::c_char,
                );
            } else {
                let mut i: libc::c_int = 1i32;
                while i < tok_cnt {
                    if addr_out.is_null()
                        && jsoneq(
                            json,
                            &mut *tok.as_mut_ptr().offset(i as isize),
                            b"email\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                    {
                        addr_out = jsondup(json, &mut *tok.as_mut_ptr().offset((i + 1i32) as isize))
                    }
                    i += 1
                }
                if addr_out.is_null() {
                    dc_log_warning(
                        context,
                        0i32,
                        b"E-mail missing in userinfo.\x00" as *const u8 as *const libc::c_char,
                    );
                }
            }
        }
    }
    free(userinfo_url as *mut libc::c_void);
    free(json as *mut libc::c_void);
    return addr_out;
}
