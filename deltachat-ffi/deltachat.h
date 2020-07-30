#ifndef __DELTACHAT_H__
#define __DELTACHAT_H__
#ifdef __cplusplus
extern "C" {
#endif


#ifndef PY_CFFI
#include <stdint.h>
#include <time.h>
#endif


typedef struct _dc_context  dc_context_t;
typedef struct _dc_array    dc_array_t;
typedef struct _dc_chatlist dc_chatlist_t;
typedef struct _dc_chat     dc_chat_t;
typedef struct _dc_msg      dc_msg_t;
typedef struct _dc_contact  dc_contact_t;
typedef struct _dc_lot      dc_lot_t;
typedef struct _dc_provider dc_provider_t;
typedef struct _dc_event    dc_event_t;
typedef struct _dc_event_emitter dc_event_emitter_t;


/**
 * @mainpage Getting started
 *
 * This document describes how to handle the Delta Chat core library.
 * For general information about Delta Chat itself,
 * see <https://delta.chat> and <https://github.com/deltachat>.
 *
 * Let's start.
 *
 * First of all, you have to **create a context object**
 * bound to a database.
 * The database is a normal sqlite-file and is created as needed:
 *
 * ~~~
 * dc_context_t* context = dc_context_new(NULL, "example.db", NULL);
 * ~~~
 *
 * After that, make sure, you can **receive events from the context**.
 * For that purpose, create an event emitter you can ask for events.
 * If there are no event, the emitter will wait until there is one,
 * so, in many situations you will do this in a thread:
 *
 * ~~~
 * void* event_handler(void* context)
 * {
 *     dc_event_emitter_t* emitter = dc_get_event_emitter(context);
 *     dc_event_t* event;
 *     while ((event = dc_get_next_event(emitter)) != NULL) {
 *         // use the event as needed, eg. dc_event_get_id() returns the type.
 *         // once you're done, unref the event to avoid memory leakage:
 *         dc_event_unref(event);
 *     }
 *     dc_event_emitter_unref(emitter);
 * }
 *
 * static pthread_t event_thread;
 * pthread_create(&event_thread, NULL, event_handler, context);
 * ~~~
 *
 * The example above uses "pthreads",
 * however, you can also use anything else for thread handling.
 * All deltachat-core-functions, unless stated otherwise, are thread-safe.
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
 * dc_configure() returns immediately,
 * the configuration itself runs in background and may take a while.
 * Once done, the #DC_EVENT_CONFIGURE_PROGRESS reports success
 * to the event_handler() you've defined above.
 *
 * The configuration result is saved in the database,
 * on subsequent starts it is not needed to call dc_configure()
 * (you can check this using dc_is_configured()).
 *
 * On a successfully configured context,
 * you can finally **connect to the servers:**
 *
 * ~~~
 * dc_start_io(context);
 * ~~~
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
 * the sending itself is done in the background.
 * If you check the testing address (bob)
 * and you should have received a normal email.
 * Answer this email in any email program with "Got it!"
 * and the IO you started above will **receive the message**.
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
 *     dc_str_unref(text);
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
 *   <https://github.com/deltachat/deltachat-core-rust/issues>
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


/**
 * @class dc_context_t
 *
 * An object representing a single account.
 *
 * Each account is linked to an IMAP/SMTP account and uses a separate
 * SQLite database for offline functionality and for account-related
 * settings.
 */

// create/open/config/information

/**
 * Create a new context object.  After creation it is usually
 * opened, connected and mails are fetched.
 *
 * @memberof dc_context_t
 * @param os_name is only for decorative use
 *     and is shown eg. in the `X-Mailer:` header
 *     in the form "Delta Chat Core <version>/<os_name>".
 *     You can give the name of the app, the operating system,
 *     the used environment and/or the version here.
 *     It is okay to give NULL, in this case `X-Mailer:` header
 *     is set to "Delta Chat Core <version>".
 * @param dbfile The file to use to store the database,
 *     something like `~/file` won't work, use absolute paths.
 * @param blobdir A directory to store the blobs in; a trailing slash is not needed.
 *     If you pass NULL or the empty string, deltachat-core creates a directory
 *     beside _dbfile_ with the same name and the suffix `-blobs`.
 * @return A context object with some public members.
 *     The object must be passed to the other context functions
 *     and must be freed using dc_context_unref() after usage.
 */
dc_context_t*   dc_context_new               (const char* os_name, const char* dbfile, const char* blobdir);


/**
 * Free a context object.
 * If app runs can only be terminated by a forced kill, this may be superfluous.
 * Before the context object is freed, connections to SMTP, IMAP and database
 * are closed. You can also do this explicitly by calling dc_close() on your own
 * before calling dc_context_unref().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 *     If NULL is given, nothing is done.
 * @return None.
 */
void            dc_context_unref             (dc_context_t* context);


/**
 * Create the event emitter that is used to receive events.
 * The library will emit various @ref DC_EVENT events as "new message", "message read" etc.
 * To get these events, you have to create an event emitter using this function
 * and call dc_get_next_event() on the emitter.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return Returns the event emitter, NULL on errors.
 *     Must be freed using dc_event_emitter_unref() after usage.
 *
 * Note: Use only one event emitter per context.
 * Having more than one event emitter running at the same time on the same context
 * will result in events randomly delivered to the one or to the other.
 */
dc_event_emitter_t* dc_get_event_emitter(dc_context_t* context);


/**
 * Get the blob directory.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return Blob directory associated with the context object, empty string if unset or on errors. NULL is never returned.
 *     The returned string must be released using dc_str_unref().
 */
char*           dc_get_blobdir               (const dc_context_t* context);


/**
 * Configure the context.  The configuration is handled by key=value pairs as:
 *
 * - `addr`         = address to display (always needed)
 * - `mail_server`  = IMAP-server, guessed if left out
 * - `mail_user`    = IMAP-username, guessed if left out
 * - `mail_pw`      = IMAP-password (always needed)
 * - `mail_port`    = IMAP-port, guessed if left out
 * - `send_server`  = SMTP-server, guessed if left out
 * - `send_user`    = SMTP-user, guessed if left out
 * - `send_pw`      = SMTP-password, guessed if left out
 * - `send_port`    = SMTP-port, guessed if left out
 * - `server_flags` = IMAP-/SMTP-flags as a combination of @ref DC_LP flags, guessed if left out
 * - `imap_certificate_checks` = how to check IMAP certificates, one of the @ref DC_CERTCK flags, defaults to #DC_CERTCK_AUTO (0)
 * - `smtp_certificate_checks` = how to check SMTP certificates, one of the @ref DC_CERTCK flags, defaults to #DC_CERTCK_AUTO (0)
 * - `displayname`  = Own name to use when sending messages.  MUAs are allowed to spread this way eg. using CC, defaults to empty
 * - `selfstatus`   = Own status to display eg. in email footers, defaults to a standard text
 * - `selfavatar`   = File containing avatar. Will immediately be copied to the 
 *                    `blobdir`; the original image will not be needed anymore.
 *                    NULL to remove the avatar.
 *                    As for `displayname` and `selfstatus`, also the avatar is sent to the recipients.
 *                    To save traffic, however, the avatar is attached only as needed
 *                    and also recoded to a reasonable size.
 * - `e2ee_enabled` = 0=no end-to-end-encryption, 1=prefer end-to-end-encryption (default)
 * - `mdns_enabled` = 0=do not send or request read receipts,
 *                    1=send and request read receipts (default)
 * - `bcc_self`     = 0=do not send a copy of outgoing messages to self (default),
 *                    1=send a copy of outgoing messages to self.
 *                    Sending messages to self is needed for a proper multi-account setup,
 *                    however, on the other hand, may lead to unwanted notifications in non-delta clients.
 * - `inbox_watch`  = 1=watch `INBOX`-folder for changes (default),
 *                    0=do not watch the `INBOX`-folder,
 *                    changes require restarting IO by calling dc_stop_io() and then dc_start_io().
 * - `sentbox_watch`= 1=watch `Sent`-folder for changes (default),
 *                    0=do not watch the `Sent`-folder,
 *                    changes require restarting IO by calling dc_stop_io() and then dc_start_io().
 * - `mvbox_watch`  = 1=watch `DeltaChat`-folder for changes (default),
 *                    0=do not watch the `DeltaChat`-folder,
 *                    changes require restarting IO by calling dc_stop_io() and then dc_start_io().
 * - `mvbox_move`   = 1=heuristically detect chat-messages
 *                    and move them to the `DeltaChat`-folder,
 *                    0=do not move chat-messages
 * - `show_emails`  = DC_SHOW_EMAILS_OFF (0)=
 *                    show direct replies to chats only (default),
 *                    DC_SHOW_EMAILS_ACCEPTED_CONTACTS (1)=
 *                    also show all mails of confirmed contacts,
 *                    DC_SHOW_EMAILS_ALL (2)=
 *                    also show mails of unconfirmed contacts in the deaddrop.
 * - `key_gen_type` = DC_KEY_GEN_DEFAULT (0)=
 *                    generate recommended key type (default),
 *                    DC_KEY_GEN_RSA2048 (1)=
 *                    generate RSA 2048 keypair
 *                    DC_KEY_GEN_ED25519 (2)=
 *                    generate Ed25519 keypair
 * - `save_mime_headers` = 1=save mime headers
 *                    and make dc_get_mime_headers() work for subsequent calls,
 *                    0=do not save mime headers (default)
 * - `delete_device_after` = 0=do not delete messages from device automatically (default),
 *                    >=1=seconds, after which messages are deleted automatically from the device.
 *                    Messages in the "saved messages" chat (see dc_chat_is_self_talk()) are skipped.
 *                    Messages are deleted whether they were seen or not, the UI should clearly point that out.
 *                    See also dc_estimate_deletion_cnt().
 * - `delete_server_after` = 0=do not delete messages from server automatically (default),
 *                    1=delete messages directly after receiving from server, mvbox is skipped.
 *                    >1=seconds, after which messages are deleted automatically from the server, mvbox is used as defined.
 *                    "Saved messages" are deleted from the server as well as
 *                    emails matching the `show_emails` settings above, the UI should clearly point that out.
 *                    See also dc_estimate_deletion_cnt().
 * - `media_quality` = DC_MEDIA_QUALITY_BALANCED (0) =
 *                    good outgoing images/videos/voice quality at reasonable sizes (default)
 *                    DC_MEDIA_QUALITY_WORSE (1)
 *                    allow worse images/videos/voice quality to gain smaller sizes,
 *                    suitable for providers or areas known to have a bad connection.
 *                    The library uses the `media_quality` setting to use different defaults
 *                    for recoding images sent with type DC_MSG_IMAGE.
 *                    If needed, recoding other file types is up to the UI.
 * - `webrtc_instance` = webrtc instance to use for videochats in the form
 *                    `[basicwebrtc:]https://example.com/subdir#roomname=$ROOM`
 *                    if the url is prefixed by `basicwebrtc`, the server is assumed to be of the type
 *                    https://github.com/cracker0dks/basicwebrtc which some UIs have native support for.
 *                    If no type is prefixed, the videochat is handled completely in a browser.
 *
 * If you want to retrieve a value, use dc_get_config().
 *
 * @memberof dc_context_t
 * @param context The context object
 * @param key The option to change, see above.
 * @param value The value to save for "key"
 * @return 0=failure, 1=success
 */
int             dc_set_config                (dc_context_t* context, const char* key, const char* value);


/**
 * Get a configuration option.
 * The configuration option is set by dc_set_config() or by the library itself.
 *
 * Beside the options shown at dc_set_config(),
 * this function can be used to query some global system values:
 *
 * - `sys.version`  = get the version string eg. as `1.2.3` or as `1.2.3special4`
 * - `sys.msgsize_max_recommended` = maximal recommended attachment size in bytes.
 *                    All possible overheads are already subtracted and this value can be used eg. for direct comparison
 *                    with the size of a file the user wants to attach. If an attachment is larger than this value,
 *                    an error (no warning as it should be shown to the user) is logged but the attachment is sent anyway.
 * - `sys.config_keys` = get a space-separated list of all config-keys available.
 *                    The config-keys are the keys that can be passed to the parameter `key` of this function.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new(). For querying system values, this can be NULL.
 * @param key The key to query.
 * @return Returns current value of "key", if "key" is unset, the default
 *     value is returned.  The returned value must be released using dc_str_unref(), NULL is never
 *     returned.  If there is an error an empty string will be returned.
 */
char*           dc_get_config                (dc_context_t* context, const char* key);


/**
 * Set stock string translation.
 *
 * The function will emit warnings if it returns an error state.
 *
 * @memberof dc_context_t
 * @param context The context object
 * @param stock_id   the integer id of the stock message (DC_STR_*)
 * @param stock_msg  the message to be used
 * @return int (==0 on error, 1 on success)
 */
int             dc_set_stock_translation(dc_context_t* context, uint32_t stock_id, const char* stock_msg);


/**
 * Set configuration values from a QR code.
 * Before this function is called, dc_check_qr() should confirm the type of the
 * QR code is DC_QR_ACCOUNT or DC_QR_WEBRTC_INSTANCE.
 *
 * Internally, the function will call dc_set_config() with the appropriate keys,
 * eg. `addr` and `mail_pw` for DC_QR_ACCOUNT
 * or `webrtc_instance` for DC_QR_WEBRTC_INSTANCE.
 *
 * @memberof dc_context_t
 * @param context The context object
 * @param qr scanned QR code
 * @return int (==0 on error, 1 on success)
 */
int             dc_set_config_from_qr   (dc_context_t* context, const char* qr);


/**
 * Get information about the context.
 *
 * The information is returned by a multi-line string
 * and contains information about the current configuration.
 *
 * If the context is not open or configured only a subset of the information
 * will be available.  There is no guarantee about which information will be
 * included when however.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return String which must be released using dc_str_unref() after usage.  Never returns NULL.
 */
char*           dc_get_info                  (dc_context_t* context);


/**
 * Get url that can be used to initiate an OAuth2 authorisation.
 *
 * If an OAuth2 authorization is possible for a given e-mail-address,
 * this function returns the URL that should be opened in a browser.
 *
 * If the user authorizes access,
 * the given redirect_uri is called by the provider.
 * It's up to the UI to handle this call.
 *
 * The provider will attach some parameters to the url,
 * most important the parameter `code` that should be set as the `mail_pw`.
 * With `server_flags` set to #DC_LP_AUTH_OAUTH2,
 * dc_configure() can be called as usual afterwards.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param addr E-mail address the user has entered.
 *     In case the user selects a different e-mail-address during
 *     authorization, this is corrected in dc_configure()
 * @param redirect_uri URL that will get `code` that is used as `mail_pw` then.
 *     Not all URLs are allowed here, however, the following should work:
 *     `chat.delta:/PATH`, `http://localhost:PORT/PATH`,
 *     `https://localhost:PORT/PATH`, `urn:ietf:wg:oauth:2.0:oob`
 *     (the latter just displays the code the user can copy+paste then)
 * @return URL that can be opened in the browser to start OAuth2.
 *     Returned strings must be released using dc_str_unref().
 *     If OAuth2 is not possible for the given e-mail-address, NULL is returned.
 */
char*           dc_get_oauth2_url            (dc_context_t* context, const char* addr, const char* redirect_uri);


// connect

/**
 * Configure a context.
 * While configuration IO must not be started, if needed stop IO using dc_stop_io() first.
 * If the context is already configured,
 * this function will try to change the configuration.
 *
 * - Before you call this function,
 *   you must set at least `addr` and `mail_pw` using dc_set_config().
 *
 * - Use `mail_user` to use a different user name than `addr`
 *   and `send_pw` to use a different password for the SMTP server.
 *
 *     - If _no_ more options are specified,
 *       the function **uses autoconfigure/autodiscover**
 *       to get the full configuration from well-known URLs.
 *
 *     - If _more_ options as `mail_server`, `mail_port`, `send_server`,
 *       `send_port`, `send_user` or `server_flags` are specified,
 *       **autoconfigure/autodiscover is skipped**.
 *
 * While dc_configure() returns immediately,
 * the started configuration-job may take a while.
 *
 * During configuration, #DC_EVENT_CONFIGURE_PROGRESS events are emmited;
 * they indicate a successful configuration as well as errors
 * and may be used to create a progress bar.
 *
 * Additional calls to dc_configure() while a config-job is running are ignored.
 * To interrupt a configuration prematurely, use dc_stop_ongoing_process();
 * this is not needed if #DC_EVENT_CONFIGURE_PROGRESS reports success.
 *
 * If #DC_EVENT_CONFIGURE_PROGRESS reports failure,
 * the core continues to use the last working configuration
 * and parameters as `addr`, `mail_pw` etc. are set to that.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return None.
 *
 * There is no need to call dc_configure() on every program start,
 * the configuration result is saved in the database
 * and you can use the connection directly:
 *
 * ~~~
 * if (!dc_is_configured(context)) {
 *     dc_configure(context);
 *     // wait for progress events
 * }
 * ~~~
 */
void            dc_configure                 (dc_context_t* context);


/**
 * Check if the context is already configured.
 *
 * Typically, for unconfigured accounts, the user is prompted
 * to enter some settings and dc_configure() is called in a thread then.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return 1=context is configured and can be used;
 *     0=context is not configured and a configuration by dc_configure() is required.
 */
int             dc_is_configured   (const dc_context_t* context);


/**
 * Start job and IMAP/SMTP tasks.
 * If IO is already running, nothing happens.
 * To check the current IO state, use dc_is_io_running().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return None
 */
void            dc_start_io     (dc_context_t* context);

/**
 * Check if IO (SMTP/IMAP/Jobs) has been started.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return 1=IO is running; 
 *   0=IO is not running.
 */
int             dc_is_io_running(const dc_context_t* context);

/**
 * Stop job and IMAP/SMTP tasks and return when they are finished. 
 * If IO is not running, nothing happens.
 * To check the current IO state, use dc_is_io_running().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return None
 */
void            dc_stop_io(dc_context_t* context);

/**
 * This function should be called when there is a hint
 * that the network is available again,
 * eg. as a response to system event reporting network availability.
 * The library will try to send pending messages out immediately.
 *
 * Moreover, to have a reliable state
 * when the app comes to foreground with network available,
 * it may be reasonable to call the function also at that moment.
 *
 * It is okay to call the function unconditionally when there is
 * network available, however, calling the function
 * _without_ having network may interfere with the backoff algorithm
 * and will led to let the jobs fail faster, with fewer retries
 * and may avoid messages being sent out.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_maybe_network             (dc_context_t* context);



/**
 * Save a keypair as the default keys for the user.
 *
 * This API is only for testing purposes and should not be used as part of a
 * normal application, use the import-export APIs instead.
 *
 * This saves a public/private keypair as the default keypair in the context.
 * It allows avoiding having to generate a secret key for unittests which need
 * one.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param addr The email address of the user.  This must match the
 *    configured_addr setting of the context as well as the UID of the key.
 * @param public_data The public key as base64.
 * @param secret_data The secret key as base64.
 * @return 1 on success, 0 on failure.
 */
int             dc_preconfigure_keypair        (dc_context_t* context, const char *addr, const char *public_data, const char *secret_data);


// handle chatlists

#define         DC_GCL_ARCHIVED_ONLY         0x01
#define         DC_GCL_NO_SPECIALS           0x02
#define         DC_GCL_ADD_ALLDONE_HINT      0x04
#define         DC_GCL_FOR_FORWARDING        0x08


/**
 * Get a list of chats.
 * The list can be filtered by query parameters.
 *
 * The list is already sorted and starts with the most recent chat in use.
 * The sorting takes care of invalid sending dates, drafts and chats without messages.
 * Clients should not try to re-sort the list as this would be an expensive action
 * and would result in inconsistencies between clients.
 *
 * To get information about each entry, use eg. dc_chatlist_get_summary().
 *
 * By default, the function adds some special entries to the list.
 * These special entries can be identified by the ID returned by dc_chatlist_get_chat_id():
 * - DC_CHAT_ID_DEADDROP (1) - this special chat is present if there are
 *   messages from addresses that have no relationship to the configured account.
 *   The last of these messages is represented by DC_CHAT_ID_DEADDROP and you can retrieve details
 *   about it with dc_chatlist_get_msg_id(). Typically, the UI asks the user "Do you want to chat with NAME?"
 *   and offers the options "Yes" (call dc_create_chat_by_msg_id()), "Never" (call dc_block_contact())
 *   or "Not now".
 *   The UI can also offer a "Close" button that calls dc_marknoticed_contact() then.
 * - DC_CHAT_ID_ARCHIVED_LINK (6) - this special chat is present if the user has
 *   archived _any_ chat using dc_set_chat_visibility(). The UI should show a link as
 *   "Show archived chats", if the user clicks this item, the UI should show a
 *   list of all archived chats that can be created by this function hen using
 *   the DC_GCL_ARCHIVED_ONLY flag.
 * - DC_CHAT_ID_ALLDONE_HINT (7) - this special chat is present
 *   if DC_GCL_ADD_ALLDONE_HINT is added to listflags
 *   and if there are only archived chats.
 *
 * @memberof dc_context_t
 * @param context The context object as returned by dc_context_new()
 * @param flags A combination of flags:
 *     - if the flag DC_GCL_ARCHIVED_ONLY is set, only archived chats are returned.
 *       if DC_GCL_ARCHIVED_ONLY is not set, only unarchived chats are returned and
 *       the pseudo-chat DC_CHAT_ID_ARCHIVED_LINK is added if there are _any_ archived
 *       chats
 *     - the flag DC_GCL_FOR_FORWARDING sorts "Saved messages" to the top of the chatlist
 *       and hides the "Device chat" and the deaddrop.
 *       typically used on forwarding, may be combined with DC_GCL_NO_SPECIALS
 *       to also hide the archive link.
 *     - if the flag DC_GCL_NO_SPECIALS is set, deaddrop and archive link are not added
 *       to the list (may be used eg. for selecting chats on forwarding, the flag is
 *       not needed when DC_GCL_ARCHIVED_ONLY is already set)
 *     - if the flag DC_GCL_ADD_ALLDONE_HINT is set, DC_CHAT_ID_ALLDONE_HINT
 *       is added as needed.
 * @param query_str An optional query for filtering the list.  Only chats matching this query
 *     are returned.  Give NULL for no filtering.
 * @param query_id An optional contact ID for filtering the list.  Only chats including this contact ID
 *     are returned.  Give 0 for no filtering.
 * @return A chatlist as an dc_chatlist_t object.
 *     On errors, NULL is returned.
 *     Must be freed using dc_chatlist_unref() when no longer used.
 *
 * See also: dc_get_chat_msgs() to get the messages of a single chat.
 */
dc_chatlist_t*  dc_get_chatlist              (dc_context_t* context, int flags, const char* query_str, uint32_t query_id);


// handle chats

/**
 * Create a normal chat or a group chat by a messages ID that comes typically
 * from the deaddrop, DC_CHAT_ID_DEADDROP (1).
 *
 * If the given message ID already belongs to a normal chat or to a group chat,
 * the chat ID of this chat is returned and no new chat is created.
 * If a new chat is created, the given message ID is moved to this chat, however,
 * there may be more messages moved to the chat from the deaddrop. To get the
 * chat messages, use dc_get_chat_msgs().
 *
 * If the user is asked before creation, he should be
 * asked whether he wants to chat with the _contact_ belonging to the message;
 * the group names may be really weird when taken from the subject of implicit
 * groups and this may look confusing.
 *
 * Moreover, this function also scales up the origin of the contact belonging
 * to the message and, depending on the contacts origin, messages from the
 * same group may be shown or not - so, all in all, it is fine to show the
 * contact name only.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param msg_id The message ID to create the chat for.
 * @return The created or reused chat ID on success. 0 on errors.
 */
uint32_t        dc_create_chat_by_msg_id     (dc_context_t* context, uint32_t msg_id);


/**
 * Create a normal chat with a single user.  To create group chats,
 * see dc_create_group_chat().
 *
 * If a chat already exists, this ID is returned, otherwise a new chat is created;
 * this new chat may already contain messages, eg. from the deaddrop, to get the
 * chat messages, use dc_get_chat_msgs().
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param contact_id The contact ID to create the chat for.  If there is already
 *     a chat with this contact, the already existing ID is returned.
 * @return The created or reused chat ID on success. 0 on errors.
 */
uint32_t        dc_create_chat_by_contact_id (dc_context_t* context, uint32_t contact_id);


/**
 * Check, if there is a normal chat with a given contact.
 * To get the chat messages, use dc_get_chat_msgs().
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param contact_id The contact ID to check.
 * @return If there is a normal chat with the given contact_id, this chat_id is
 *     returned.  If there is no normal chat with the contact_id, the function
 *     returns 0.
 */
uint32_t        dc_get_chat_id_by_contact_id (dc_context_t* context, uint32_t contact_id);


/**
 * Prepare a message for sending.
 *
 * Call this function if the file to be sent is still in creation.
 * Once you're done with creating the file, call dc_send_msg() as usual
 * and the message will really be sent.
 *
 * This is useful as the user can already send the next messages while
 * e.g. the recoding of a video is not yet finished. Or the user can even forward
 * the message with the file being still in creation to other groups.
 *
 * Files being sent with the increation-method must be placed in the
 * blob directory, see dc_get_blobdir().
 * If the increation-method is not used - which is probably the normal case -
 * dc_send_msg() copies the file to the blob directory if it is not yet there.
 * To distinguish the two cases, msg->state must be set properly. The easiest
 * way to ensure this is to re-use the same object for both calls.
 *
 * Example:
 * ~~~
 * char* blobdir = dc_get_blobdir(context);
 * char* file_to_send = mprintf("%s/%s", blobdir, "send.mp4")
 *
 * dc_msg_t* msg = dc_msg_new(context, DC_MSG_VIDEO);
 * dc_msg_set_file(msg, file_to_send, NULL);
 * dc_prepare_msg(context, chat_id, msg);
 *
 * // ... create the file ...
 *
 * dc_send_msg(context, chat_id, msg);
 *
 * dc_msg_unref(msg);
 * free(file_to_send);
 * dc_str_unref(file_to_send);
 * ~~~
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id Chat ID to send the message to.
 * @param msg Message object to send to the chat defined by the chat ID.
 *     On succcess, msg_id and state of the object are set up,
 *     The function does not take ownership of the object,
 *     so you have to free it using dc_msg_unref() as usual.
 * @return The ID of the message that is being prepared.
 */
uint32_t        dc_prepare_msg               (dc_context_t* context, uint32_t chat_id, dc_msg_t* msg);


/**
 * Send a message defined by a dc_msg_t object to a chat.
 *
 * Sends the event #DC_EVENT_MSGS_CHANGED on succcess.
 * However, this does not imply, the message really reached the recipient -
 * sending may be delayed eg. due to network problems. However, from your
 * view, you're done with the message. Sooner or later it will find its way.
 *
 * Example:
 * ~~~
 * dc_msg_t* msg = dc_msg_new(context, DC_MSG_IMAGE);
 *
 * dc_msg_set_file(msg, "/file/to/send.jpg", NULL);
 * dc_send_msg(context, chat_id, msg);
 *
 * dc_msg_unref(msg);
 * ~~~
 *
 * If you send images with the DC_MSG_IMAGE type,
 * they will be recoded to a reasonable size before sending, if possible
 * (cmp the dc_set_config()-option `media_quality`).
 * If that fails, is not possible, or the image is already small enough, the image is sent as original.
 * If you want images to be always sent as the original file, use the DC_MSG_FILE type.
 *
 * Videos and other file types are currently not recoded by the library,
 * with dc_prepare_msg(), however, you can do that from the UI.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id Chat ID to send the message to.
 *     If dc_prepare_msg() was called before, this parameter can be 0.
 * @param msg Message object to send to the chat defined by the chat ID.
 *     On succcess, msg_id of the object is set up,
 *     The function does not take ownership of the object,
 *     so you have to free it using dc_msg_unref() as usual.
 * @return The ID of the message that is about to be sent. 0 in case of errors.
 */
uint32_t        dc_send_msg                  (dc_context_t* context, uint32_t chat_id, dc_msg_t* msg);

/**
 * Send a message defined by a dc_msg_t object to a chat, synchronously.
 * This bypasses the IO scheduler and creates its own SMTP connection. Which means
 * this is useful when the scheduler is not running.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id Chat ID to send the message to.
 *     If dc_prepare_msg() was called before, this parameter can be 0.
 * @param msg Message object to send to the chat defined by the chat ID.
 *     On succcess, msg_id of the object is set up,
 *     The function does not take ownership of the object,
 *     so you have to free it using dc_msg_unref() as usual.
 * @return The ID of the message that is about to be sent. 0 in case of errors.
 */
uint32_t        dc_send_msg_sync                  (dc_context_t* context, uint32_t chat_id, dc_msg_t* msg);


/**
 * Send a simple text message a given chat.
 *
 * Sends the event #DC_EVENT_MSGS_CHANGED on succcess.
 * However, this does not imply, the message really reached the recipient -
 * sending may be delayed eg. due to network problems. However, from your
 * view, you're done with the message. Sooner or later it will find its way.
 *
 * See also dc_send_msg().
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id Chat ID to send the text message to.
 * @param text_to_send Text to send to the chat defined by the chat ID.
 *     Passing an empty text here causes an empty text to be sent,
 *     it's up to the caller to handle this if undesired.
 *     Passing NULL as the text causes the function to return 0.
 * @return The ID of the message that is about being sent.
 */
uint32_t        dc_send_text_msg             (dc_context_t* context, uint32_t chat_id, const char* text_to_send);


/**
 * Send invitation to a videochat.
 *
 * This function reads the `webrtc_instance` config value,
 * may check that the server is working in some way
 * and creates a unique room for this chat, if needed doing a TOKEN roundtrip for that.
 *
 * After that, the function sends out a message that contains information to join the room:
 *
 * - To allow non-delta-clients to join the chat,
 *   the message contains a text-area with some descriptive text
 *   and a url that can be opened in a supported browser to join the videochat
 *
 * - delta-clients can get all information needed from
 *   the message object, using eg.
 *   dc_msg_get_videochat_url() and check dc_msg_get_viewtype() for #DC_MSG_VIDEOCHAT_INVITATION
 *
 * dc_send_videochat_invitation() is blocking and may take a while,
 * so the UIs will typically call the function from within a thread.
 * Moreover, UIs will typically enter the room directly without an additional click on the message,
 * for this purpose, the function returns the message-id directly.
 *
 * As for other messages sent, this function
 * sends the event #DC_EVENT_MSGS_CHANGED on succcess, the message has a delivery state, and so on.
 * The recipient will get noticed by the call as usual by #DC_EVENT_INCOMING_MSG or #DC_EVENT_MSGS_CHANGED,
 * However, UIs might some things differently, eg. play a different sound.
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param chat_id The chat to start a videochat for.
 * @return The id if the message sent out
 *     or 0 for errors.
 */
uint32_t dc_send_videochat_invitation (dc_context_t* context, uint32_t chat_id);


/**
 * Save a draft for a chat in the database.
 *
 * The UI should call this function if the user has prepared a message
 * and exits the compose window without clicking the "send" button before.
 * When the user later opens the same chat again,
 * the UI can load the draft using dc_get_draft()
 * allowing the user to continue editing and sending.
 *
 * Drafts are considered when sorting messages
 * and are also returned eg. by dc_chatlist_get_summary().
 *
 * Each chat can have its own draft but only one draft per chat is possible.
 *
 * If the draft is modified, an #DC_EVENT_MSGS_CHANGED will be sent.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID to save the draft for.
 * @param msg The message to save as a draft.
 *     Existing draft will be overwritten.
 *     NULL deletes the existing draft, if any, without sending it.
 *     Currently, also non-text-messages
 *     will delete the existing drafts.
 * @return None.
 */
void            dc_set_draft                 (dc_context_t* context, uint32_t chat_id, dc_msg_t* msg);


/**
 * Add a message to the device-chat.
 * Device-messages usually contain update information
 * and some hints that are added during the program runs, multi-device etc.
 * The device-message may be defined by a label;
 * if a message with the same label was added or skipped before,
 * the message is not added again, even if the message was deleted in between.
 * If needed, the device-chat is created before.
 *
 * Sends the event #DC_EVENT_MSGS_CHANGED on success.
 * To check, if a given chat is a device-chat, see dc_chat_is_device_talk()
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param label A unique name for the message to add.
 *     The label is typically not displayed to the user and
 *     must be created from the characters `A-Z`, `a-z`, `0-9`, `_` or `-`.
 *     If you pass NULL here, the message is added unconditionally.
 * @param msg Message to be added to the device-chat.
 *     The message appears to the user as an incoming message.
 *     If you pass NULL here, only the given label will be added
 *     and block adding messages with that label in the future.
 * @return The ID of the just added message,
 *     if the message was already added or no message to add is given, 0 is returned.
 *
 * Example:
 * ~~~
 * dc_msg_t* welcome_msg = dc_msg_new(DC_MSG_TEXT);
 * dc_msg_set_text(welcome_msg, "great that you give this app a try!");
 *
 * dc_msg_t* changelog_msg = dc_msg_new(DC_MSG_TEXT);
 * dc_msg_set_text(changelog_msg, "we have added 3 new emojis :)");
 *
 * if (dc_add_device_msg(context, "welcome", welcome_msg)) {
 *     // do not add the changelog on a new installations -
 *     // not now and not when this code is executed again
 *     dc_add_device_msg(context, "update-123", NULL);
 * } else {
 *     // welcome message was not added now, this is an oder installation,
 *     // add a changelog
 *     dc_add_device_msg(context, "update-123", changelog_msg);
 * }
 *
 * dc_msg_unref(changelog_msg);
 * dc_msg_unref(welome_msg);
 * ~~~
 */
uint32_t        dc_add_device_msg            (dc_context_t* context, const char* label, dc_msg_t* msg);


/**
 * Init device-messages and saved-messages chat.
 * This function adds the device-chat and saved-messages chat
 * and adds one or more welcome or update-messages.
 * The ui can add messages on its own using dc_add_device_msg() -
 * for ordering, either before or after or even without calling this function.
 *
 * Chat and message creation is done only once.
 * So if the user has manually deleted things, they won't be re-created
 * (however, not seen device messages are added and may re-create the device-chat).
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_update_device_chats       (dc_context_t* context);


/**
 * Check if a device-message with a given label was ever added.
 * Device-messages can be added dc_add_device_msg().
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param label Label of the message to check.
 * @return 1=A message with this label was added at some point,
 *     0=A message with this label was never added.
 */
int             dc_was_device_msg_ever_added (dc_context_t* context, const char* label);


/**
 * Get draft for a chat, if any.
 * See dc_set_draft() for more details about drafts.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID to get the draft for.
 * @return Message object.
 *     Can be passed directly to dc_send_msg().
 *     Must be freed using dc_msg_unref() after usage.
 *     If there is no draft, NULL is returned.
 */
dc_msg_t*       dc_get_draft                 (dc_context_t* context, uint32_t chat_id);


#define         DC_GCM_ADDDAYMARKER          0x01


/**
 * Get all message IDs belonging to a chat.
 *
 * The list is already sorted and starts with the oldest message.
 * Clients should not try to re-sort the list as this would be an expensive action
 * and would result in inconsistencies between clients.
 *
 * Optionally, some special markers added to the ID-array may help to
 * implement virtual lists.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The chat ID of which the messages IDs should be queried.
 * @param flags If set to DC_GCM_ADDDAYMARKER, the marker DC_MSG_ID_DAYMARKER will
 *     be added before each day (regarding the local timezone).  Set this to 0 if you do not want this behaviour.
 *     To get the concrete time of the marker, use dc_array_get_timestamp().
 * @param marker1before An optional message ID.  If set, the id DC_MSG_ID_MARKER1 will be added just
 *   before the given ID in the returned array.  Set this to 0 if you do not want this behaviour.
 * @return Array of message IDs, must be dc_array_unref()'d when no longer used.
 */
dc_array_t*     dc_get_chat_msgs             (dc_context_t* context, uint32_t chat_id, uint32_t flags, uint32_t marker1before);


/**
 * Get the total number of messages in a chat.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to count the messages for.
 * @return Number of total messages in the given chat. 0 for errors or empty chats.
 */
int             dc_get_msg_cnt               (dc_context_t* context, uint32_t chat_id);


/**
 * Get the number of _fresh_ messages in a chat.  Typically used to implement
 * a badge with a number in the chatlist.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to count the messages for.
 * @return Number of fresh messages in the given chat. 0 for errors or if there are no fresh messages.
 */
int             dc_get_fresh_msg_cnt         (dc_context_t* context, uint32_t chat_id);



/**
 * Estimate the number of messages that will be deleted
 * by the dc_set_config()-options `delete_device_after` or `delete_server_after`.
 * This is typically used to show the estimated impact to the user before actually enabling ephemeral messages.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param from_server 1=Estimate deletion count for server, 0=Estimate deletion count for device
 * @param seconds Count messages older than the given number of seconds.
 * @return Number of messages that are older than the given number of seconds.
 *     This includes emails downloaded due to the `show_emails` option.
 *     Messages in the "saved messages" folder are not counted as they will not be deleted automatically.
 */
int             dc_estimate_deletion_cnt    (dc_context_t* context, int from_server, int64_t seconds);

/**
 * Returns the message IDs of all _fresh_ messages of any chat.
 * Typically used for implementing notification summaries.
 * The list is already sorted and starts with the most recent fresh message.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @return Array of message IDs, must be dc_array_unref()'d when no longer used.
 *     On errors, the list is empty. NULL is never returned.
 */
dc_array_t*     dc_get_fresh_msgs            (dc_context_t* context);


/**
 * Mark all messages in a chat as _noticed_.
 * _Noticed_ messages are no longer _fresh_ and do not count as being unseen
 * but are still waiting for being marked as "seen" using dc_markseen_msgs()
 * (IMAP/MDNs is not done for noticed messages).
 *
 * Calling this function usually results in the event #DC_EVENT_MSGS_CHANGED.
 * See also dc_marknoticed_all_chats(), dc_marknoticed_contact() and dc_markseen_msgs().
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The chat ID of which all messages should be marked as being noticed.
 * @return None.
 */
void            dc_marknoticed_chat          (dc_context_t* context, uint32_t chat_id);


/**
 * Same as dc_marknoticed_chat() but for _all_ chats.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @return None.
 */
void            dc_marknoticed_all_chats     (dc_context_t* context);


/**
 * Returns all message IDs of the given types in a chat.
 * Typically used to show a gallery.
 * The result must be dc_array_unref()'d
 *
 * The list is already sorted and starts with the oldest message.
 * Clients should not try to re-sort the list as this would be an expensive action
 * and would result in inconsistencies between clients.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The chat ID to get all messages with media from.
 * @param msg_type Specify a message type to query here, one of the @ref DC_MSG constants.
 * @param msg_type2 Alternative message type to search for. 0 to skip.
 * @param msg_type3 Alternative message type to search for. 0 to skip.
 * @return An array with messages from the given chat ID that have the wanted message types.
 */
dc_array_t*     dc_get_chat_media            (dc_context_t* context, uint32_t chat_id, int msg_type, int msg_type2, int msg_type3);


/**
 * Search next/previous message based on a given message and a list of types.
 * The
 * Typically used to implement the "next" and "previous" buttons
 * in a gallery or in a media player.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param msg_id  This is the current message
 *     from which the next or previous message should be searched.
 * @param dir 1=get the next message, -1=get the previous one.
 * @param msg_type Message type to search for.
 *     If 0, the message type from curr_msg_id is used.
 * @param msg_type2 Alternative message type to search for. 0 to skip.
 * @param msg_type3 Alternative message type to search for. 0 to skip.
 * @return Returns the message ID that should be played next.
 *     The returned message is in the same chat as the given one
 *     and has one of the given types.
 *     Typically, this result is passed again to dc_get_next_media()
 *     later on the next swipe.
 *     If there is not next/previous message, the function returns 0.
 */
uint32_t        dc_get_next_media            (dc_context_t* context, uint32_t msg_id, int dir, int msg_type, int msg_type2, int msg_type3);


/**
 * Set chat visibility to pinned, archived or normal.
 *
 * Calling this function usually results in the event #DC_EVENT_MSGS_CHANGED
 * See @ref DC_CHAT_VISIBILITY for detailed information about the visibilities.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to change the visibility for.
 * @param visibility one of @ref DC_CHAT_VISIBILITY
 * @return None.
 */
void            dc_set_chat_visibility       (dc_context_t* context, uint32_t chat_id, int visibility);


/**
 * Delete a chat.
 *
 * Messages are deleted from the device and the chat database entry is deleted.
 * After that, the event #DC_EVENT_MSGS_CHANGED is posted.
 *
 * Things that are _not_ done implicitly:
 *
 * - Messages are **not deleted from the server**.
 * - The chat or the contact is **not blocked**, so new messages from the user/the group may appear
 *   and the user may create the chat again.
 * - **Groups are not left** - this would
 *   be unexpected as (1) deleting a normal chat also does not prevent new mails
 *   from arriving, (2) leaving a group requires sending a message to
 *   all group members - especially for groups not used for a longer time, this is
 *   really unexpected when deletion results in contacting all members again,
 *   (3) only leaving groups is also a valid usecase.
 *
 * To leave a chat explicitly, use dc_remove_contact_from_chat() with
 * chat_id=DC_CONTACT_ID_SELF)
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to delete.
 * @return None.
 */
void            dc_delete_chat               (dc_context_t* context, uint32_t chat_id);


/**
 * Get contact IDs belonging to a chat.
 *
 * - for normal chats, the function always returns exactly one contact,
 *   DC_CONTACT_ID_SELF is returned only for SELF-chats.
 *
 * - for group chats all members are returned, DC_CONTACT_ID_SELF is returned
 *   explicitly as it may happen that oneself gets removed from a still existing
 *   group
 *
 * - for the deaddrop, the list is empty
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id Chat ID to get the belonging contact IDs for.
 * @return An array of contact IDs belonging to the chat; must be freed using dc_array_unref() when done.
 */
dc_array_t*     dc_get_chat_contacts         (dc_context_t* context, uint32_t chat_id);

/**
 * Get the chat's ephemeral message timer.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID.
 *
 * @return ephemeral timer value in seconds, 0 if the timer is disabled or if there is an error
 */
uint32_t dc_get_chat_ephemeral_timer (dc_context_t* context, uint32_t chat_id);

/**
 * Search messages containing the given query string.
 * Searching can be done globally (chat_id=0) or in a specified chat only (chat_id
 * set).
 *
 * Global chat results are typically displayed using dc_msg_get_summary(), chat
 * search results may just hilite the corresponding messages and present a
 * prev/next button.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id ID of the chat to search messages in.
 *     Set this to 0 for a global search.
 * @param query The query to search for.
 * @return An array of message IDs. Must be freed using dc_array_unref() when no longer needed.
 *     If nothing can be found, the function returns NULL.
 */
dc_array_t*     dc_search_msgs               (dc_context_t* context, uint32_t chat_id, const char* query);


/**
 * Get chat object by a chat ID.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to get the chat object for.
 * @return A chat object of the type dc_chat_t,
 *     must be freed using dc_chat_unref() when done.
 *     On errors, NULL is returned.
 */
dc_chat_t*      dc_get_chat                  (dc_context_t* context, uint32_t chat_id);


// handle group chats

/**
 * Create a new group chat.
 *
 * After creation,
 * the draft of the chat is set to a default text,
 * the group has one member with the ID DC_CONTACT_ID_SELF
 * and is in _unpromoted_ state.
 * This means, you can add or remove members, change the name,
 * the group image and so on without messages being sent to all group members.
 *
 * This changes as soon as the first message is sent to the group members
 * and the group becomes _promoted_.
 * After that, all changes are synced with all group members
 * by sending status message.
 *
 * To check, if a chat is still unpromoted, you dc_chat_is_unpromoted().
 * This may be useful if you want to show some help for just created groups.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param verified If set to 1 the function creates a secure verified group.
 *     Only secure-verified members are allowed in these groups
 *     and end-to-end-encryption is always enabled.
 * @param name The name of the group chat to create.
 *     The name may be changed later using dc_set_chat_name().
 *     To find out the name of a group later, see dc_chat_get_name()
 * @return The chat ID of the new group chat, 0 on errors.
 */
uint32_t        dc_create_group_chat         (dc_context_t* context, int verified, const char* name);


/**
 * Check if a given contact ID is a member of a group chat.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID to check.
 * @param contact_id The contact ID to check.  To check if yourself is member
 *     of the chat, pass DC_CONTACT_ID_SELF (1) here.
 * @return 1=contact ID is member of chat ID, 0=contact is not in chat
 */
int             dc_is_contact_in_chat        (dc_context_t* context, uint32_t chat_id, uint32_t contact_id);


/**
 * Add a member to a group.
 *
 * If the group is already _promoted_ (any message was sent to the group),
 * all group members are informed by a special status message that is sent automatically by this function.
 *
 * If the group is a verified group, only verified contacts can be added to the group.
 *
 * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID to add the contact to.  Must be a group chat.
 * @param contact_id The contact ID to add to the chat.
 * @return 1=member added to group, 0=error
 */
int             dc_add_contact_to_chat       (dc_context_t* context, uint32_t chat_id, uint32_t contact_id);


/**
 * Remove a member from a group.
 *
 * If the group is already _promoted_ (any message was sent to the group),
 * all group members are informed by a special status message that is sent automatically by this function.
 *
 * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID to remove the contact from.  Must be a group chat.
 * @param contact_id The contact ID to remove from the chat.
 * @return 1=member removed from group, 0=error
 */
int             dc_remove_contact_from_chat  (dc_context_t* context, uint32_t chat_id, uint32_t contact_id);


/**
 * Set group name.
 *
 * If the group is already _promoted_ (any message was sent to the group),
 * all group members are informed by a special status message that is sent automatically by this function.
 *
 * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
 *
 * @memberof dc_context_t
 * @param chat_id The chat ID to set the name for.  Must be a group chat.
 * @param name New name of the group.
 * @param context The context as created by dc_context_new().
 * @return 1=success, 0=error
 */
int             dc_set_chat_name             (dc_context_t* context, uint32_t chat_id, const char* name);

/**
 * Set the chat's ephemeral message timer.
 *
 * This timer is applied to all messages in a chat and starts when the
 * message is read. The setting is synchronized to all clients
 * participating in a chat.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID to set the ephemeral message timer for.
 * @param timer The timer value in seconds or 0 to disable the timer.
 *
 * @return 1=success, 0=error
 */
int dc_set_chat_ephemeral_timer (dc_context_t* context, uint32_t chat_id, uint32_t timer);

/**
 * Set group profile image.
 *
 * If the group is already _promoted_ (any message was sent to the group),
 * all group members are informed by a special status message that is sent automatically by this function.
 *
 * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
 *
 * To find out the profile image of a chat, use dc_chat_get_profile_image()
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param chat_id The chat ID to set the image for.
 * @param image Full path of the image to use as the group image. The image will immediately be copied to the 
 *     `blobdir`; the original image will not be needed anymore.
 *      If you pass NULL here, the group image is deleted (for promoted groups, all members are informed about 
 *      this change anyway).
 * @return 1=success, 0=error
 */
int             dc_set_chat_profile_image    (dc_context_t* context, uint32_t chat_id, const char* image);



/**
 * Set mute duration of a chat.
 *
 * The ui can then call dc_chat_is_muted() when receiving a new message to decide whether it should trigger an notification.
 *
 * Sends out #DC_EVENT_CHAT_MODIFIED.
 *
 * @memberof dc_context_t
 * @param chat_id The chat ID to set the mute duration.
 * @param duration The duration (0 for no mute, -1 for forever mute, everything else is is the relative mute duration from now in seconds)
 * @param context The context as created by dc_context_new().
 * @return 1=success, 0=error
 */
int             dc_set_chat_mute_duration             (dc_context_t* context, uint32_t chat_id, int64_t duration);

// handle messages

/**
 * Get an informational text for a single message. The text is multiline and may
 * contain eg. the raw text of the message.
 *
 * The max. text returned is typically longer (about 100000 characters) than the
 * max. text returned by dc_msg_get_text() (about 30000 characters).
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param msg_id The message id for which information should be generated
 * @return Text string, must be released using dc_str_unref() after usage
 */
char*           dc_get_msg_info              (dc_context_t* context, uint32_t msg_id);


/**
 * Get the raw mime-headers of the given message.
 * Raw headers are saved for incoming messages
 * only if `dc_set_config(context, "save_mime_headers", "1")`
 * was called before.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param msg_id The message id, must be the id of an incoming message.
 * @return Raw headers as a multi-line string, must be released using dc_str_unref() after usage.
 *     Returns NULL if there are no headers saved for the given message,
 *     eg. because of save_mime_headers is not set
 *     or the message is not incoming.
 */
char*           dc_get_mime_headers          (dc_context_t* context, uint32_t msg_id);


/**
 * Delete messages. The messages are deleted on the current device and
 * on the IMAP server.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new()
 * @param msg_ids an array of uint32_t containing all message IDs that should be deleted
 * @param msg_cnt The number of messages IDs in the msg_ids array
 * @return None.
 */
void            dc_delete_msgs               (dc_context_t* context, const uint32_t* msg_ids, int msg_cnt);

/*
 * Empty IMAP server folder: delete all messages.
 * Deprecated, use dc_set_config() with the key "delete_server_after" instead.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new()
 * @param flags What to delete, a combination of the @ref DC_EMPTY flags
 * @return None.
 */
void            dc_empty_server              (dc_context_t* context, uint32_t flags);


/**
 * Forward messages to another chat.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new()
 * @param msg_ids An array of uint32_t containing all message IDs that should be forwarded
 * @param msg_cnt The number of messages IDs in the msg_ids array
 * @param chat_id The destination chat ID.
 * @return None.
 */
void            dc_forward_msgs              (dc_context_t* context, const uint32_t* msg_ids, int msg_cnt, uint32_t chat_id);


/**
 * Mark all messages sent by the given contact
 * as _noticed_.  See also dc_marknoticed_chat() and
 * dc_markseen_msgs()
 *
 * Calling this function usually results in the event #DC_EVENT_MSGS_CHANGED.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new()
 * @param contact_id The contact ID of which all messages should be marked as noticed.
 * @return None.
 */
void            dc_marknoticed_contact       (dc_context_t* context, uint32_t contact_id);


/**
 * Mark a message as _seen_, updates the IMAP state and
 * sends MDNs. If the message is not in a real chat (eg. a contact request), the
 * message is only marked as NOTICED and no IMAP/MDNs is done.  See also
 * dc_marknoticed_chat() and dc_marknoticed_contact()
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param msg_ids An array of uint32_t containing all the messages IDs that should be marked as seen.
 * @param msg_cnt The number of message IDs in msg_ids.
 * @return None.
 */
void            dc_markseen_msgs             (dc_context_t* context, const uint32_t* msg_ids, int msg_cnt);


/**
 * Star/unstar messages by setting the last parameter to 0 (unstar) or 1 (star).
 * Starred messages are collected in a virtual chat that can be shown using
 * dc_get_chat_msgs() using the chat_id DC_CHAT_ID_STARRED.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new()
 * @param msg_ids An array of uint32_t message IDs defining the messages to star or unstar
 * @param msg_cnt The number of IDs in msg_ids
 * @param star 0=unstar the messages in msg_ids, 1=star them
 * @return None.
 */
void            dc_star_msgs                 (dc_context_t* context, const uint32_t* msg_ids, int msg_cnt, int star);


/**
 * Get a single message object of the type dc_msg_t.
 * For a list of messages in a chat, see dc_get_chat_msgs()
 * For a list or chats, see dc_get_chatlist()
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param msg_id The message ID for which the message object should be created.
 * @return A dc_msg_t message object.
 *     On errors, NULL is returned.
 *     When done, the object must be freed using dc_msg_unref().
 */
dc_msg_t*       dc_get_msg                   (dc_context_t* context, uint32_t msg_id);


// handle contacts

/**
 * Rough check if a string may be a valid e-mail address.
 * The function checks if the string contains a minimal amount of characters
 * before and after the `@` and `.` characters.
 *
 * To check if a given address is a contact in the contact database
 * use dc_lookup_contact_id_by_addr().
 *
 * @memberof dc_context_t
 * @param addr The e-mail-address to check.
 * @return 1=address may be a valid e-mail address,
 *     0=address won't be a valid e-mail address
 */
int             dc_may_be_valid_addr         (const char* addr);


/**
 * Check if an e-mail address belongs to a known and unblocked contact.
 * To get a list of all known and unblocked contacts, use dc_get_contacts().
 *
 * To validate an e-mail address independently of the contact database
 * use dc_may_be_valid_addr().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param addr The e-mail-address to check.
 * @return Contact ID of the contact belonging to the e-mail-address
 *     or 0 if there is no contact that is or was introduced by an accepted contact.  
 */
uint32_t        dc_lookup_contact_id_by_addr (dc_context_t* context, const char* addr);


/**
 * Add a single contact as a result of an _explicit_ user action.
 *
 * We assume, the contact name, if any, is entered by the user and is used "as is" therefore,
 * normalize() is _not_ called for the name. If the contact is blocked, it is unblocked.
 *
 * To add a number of contacts, see dc_add_address_book() which is much faster for adding
 * a bunch of addresses.
 *
 * May result in a #DC_EVENT_CONTACTS_CHANGED event.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param name Name of the contact to add. If you do not know the name belonging
 *     to the address, you can give NULL here.
 * @param addr E-mail-address of the contact to add. If the email address
 *     already exists, the name is updated and the origin is increased to
 *     "manually created".
 * @return Contact ID of the created or reused contact.
 */
uint32_t        dc_create_contact            (dc_context_t* context, const char* name, const char* addr);


#define         DC_GCL_VERIFIED_ONLY         0x01
#define         DC_GCL_ADD_SELF              0x02


/**
 * Add a number of contacts.
 *
 * Typically used to add the whole address book from the OS. As names here are typically not
 * well formatted, we call normalize() for each name given.
 *
 * No email-address is added twice.
 * Trying to add email-addresses that are already in the contact list,
 * results in updating the name unless the name was changed manually by the user.
 * If any email-address or any name is really updated,
 * the event #DC_EVENT_CONTACTS_CHANGED is sent.
 *
 * To add a single contact entered by the user, you should prefer dc_create_contact(),
 * however, for adding a bunch of addresses, this function is _much_ faster.
 *
 * @memberof dc_context_t
 * @param context the context object as created by dc_context_new().
 * @param addr_book A multi-line string in the format
 *     `Name one\nAddress one\nName two\nAddress two`.
 *      If an email address already exists, the name is updated
 *      unless it was edited manually by dc_create_contact() before.
 * @return The number of modified or added contacts.
 */
int             dc_add_address_book          (dc_context_t* context, const char* addr_book);


/**
 * Returns known and unblocked contacts.
 *
 * To get information about a single contact, see dc_get_contact().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param flags A combination of flags:
 *     - if the flag DC_GCL_ADD_SELF is set, SELF is added to the list unless filtered by other parameters
 *     - if the flag DC_GCL_VERIFIED_ONLY is set, only verified contacts are returned.
 *       if DC_GCL_VERIFIED_ONLY is not set, verified and unverified contacts are returned.
 * @param query A string to filter the list.  Typically used to implement an
 *     incremental search.  NULL for no filtering.
 * @return An array containing all contact IDs.  Must be dc_array_unref()'d
 *     after usage.
 */
dc_array_t*     dc_get_contacts              (dc_context_t* context, uint32_t flags, const char* query);


/**
 * Get the number of blocked contacts.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return The number of blocked contacts.
 */
int             dc_get_blocked_cnt           (dc_context_t* context);


/**
 * Get blocked contacts.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return An array containing all blocked contact IDs.  Must be dc_array_unref()'d
 *     after usage.
 */
dc_array_t*     dc_get_blocked_contacts      (dc_context_t* context);


/**
 * Block or unblock a contact.
 * May result in a #DC_EVENT_CONTACTS_CHANGED event.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param contact_id The ID of the contact to block or unblock.
 * @param block 1=block contact, 0=unblock contact
 * @return None.
 */
void            dc_block_contact             (dc_context_t* context, uint32_t contact_id, int block);


/**
 * Get encryption info for a contact.
 * Get a multi-line encryption info, containing your fingerprint and the
 * fingerprint of the contact, used eg. to compare the fingerprints for a simple out-of-band verification.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param contact_id ID of the contact to get the encryption info for.
 * @return Multi-line text, must be released using dc_str_unref() after usage.
 */
char*           dc_get_contact_encrinfo      (dc_context_t* context, uint32_t contact_id);


/**
 * Delete a contact.  The contact is deleted from the local device.  It may happen that this is not
 * possible as the contact is in use.  In this case, the contact can be blocked.
 *
 * May result in a #DC_EVENT_CONTACTS_CHANGED event.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param contact_id ID of the contact to delete.
 * @return 1=success, 0=error
 */
int             dc_delete_contact            (dc_context_t* context, uint32_t contact_id);


/**
 * Get a single contact object.  For a list, see eg. dc_get_contacts().
 *
 * For contact DC_CONTACT_ID_SELF (1), the function returns sth.
 * like "Me" in the selected language and the email address
 * defined by dc_set_config().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param contact_id ID of the contact to get the object for.
 * @return The contact object, must be freed using dc_contact_unref() when no
 *     longer used.  NULL on errors.
 */
dc_contact_t*   dc_get_contact               (dc_context_t* context, uint32_t contact_id);


// import/export and tools

#define         DC_IMEX_EXPORT_SELF_KEYS      1 // param1 is a directory where the keys are written to
#define         DC_IMEX_IMPORT_SELF_KEYS      2 // param1 is a directory where the keys are searched in and read from
#define         DC_IMEX_EXPORT_BACKUP        11 // param1 is a directory where the backup is written to
#define         DC_IMEX_IMPORT_BACKUP        12 // param1 is the file with the backup to import


/**
 * Import/export things.
 * What to do is defined by the _what_ parameter which may be one of the following:
 *
 * - **DC_IMEX_EXPORT_BACKUP** (11) - Export a backup to the directory given as `param1`.
 *   The backup contains all contacts, chats, images and other data and device independent settings.
 *   The backup does not contain device dependent settings as ringtones or LED notification settings.
 *   The name of the backup is typically `delta-chat.<day>.bak`, if more than one backup is create on a day,
 *   the format is `delta-chat.<day>-<number>.bak`
 *
 * - **DC_IMEX_IMPORT_BACKUP** (12) - `param1` is the file (not: directory) to import. The file is normally
 *   created by DC_IMEX_EXPORT_BACKUP and detected by dc_imex_has_backup(). Importing a backup
 *   is only possible as long as the context is not configured or used in another way.
 *
 * - **DC_IMEX_EXPORT_SELF_KEYS** (1) - Export all private keys and all public keys of the user to the
 *   directory given as `param1`.  The default key is written to the files `public-key-default.asc`
 *   and `private-key-default.asc`, if there are more keys, they are written to files as
 *   `public-key-<id>.asc` and `private-key-<id>.asc`
 *
 * - **DC_IMEX_IMPORT_SELF_KEYS** (2) - Import private keys found in the directory given as `param1`.
 *   The last imported key is made the default keys unless its name contains the string `legacy`.  Public keys are not imported.
 *
 * While dc_imex() returns immediately, the started job may take a while,
 * you can stop it using dc_stop_ongoing_process(). During execution of the job,
 * some events are sent out:
 *
 * - A number of #DC_EVENT_IMEX_PROGRESS events are sent and may be used to create
 *   a progress bar or stuff like that. Moreover, you'll be informed when the imex-job is done.
 *
 * - For each file written on export, the function sends #DC_EVENT_IMEX_FILE_WRITTEN
 *
 * Only one import-/export-progress can run at the same time.
 * To cancel an import-/export-progress, use dc_stop_ongoing_process().
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param what One of the DC_IMEX_* constants.
 * @param param1 Meaning depends on the DC_IMEX_* constants. If this parameter is a directory, it should not end with
 *     a slash (otherwise you'll get double slashes when receiving #DC_EVENT_IMEX_FILE_WRITTEN). Set to NULL if not used.
 * @param param2 Meaning depends on the DC_IMEX_* constants. Set to NULL if not used.
 * @return None.
 */
void            dc_imex                      (dc_context_t* context, int what, const char* param1, const char* param2);


/**
 * Check if there is a backup file.
 * May only be used on fresh installations (eg. dc_is_configured() returns 0).
 *
 * Example:
 *
 * ~~~
 * char dir[] = "/dir/to/search/backups/in";
 *
 * void ask_user_for_credentials()
 * {
 *     // - ask the user for email and password
 *     // - save them using dc_set_config()
 * }
 *
 * int ask_user_whether_to_import()
 * {
 *     // - inform the user that we've found a backup
 *     // - ask if he want to import it
 *     // - return 1 to import, 0 to skip
 *     return 1;
 * }
 *
 * if (!dc_is_configured(context))
 * {
 *     char* file = NULL;
 *     if ((file=dc_imex_has_backup(context, dir))!=NULL && ask_user_whether_to_import())
 *     {
 *         dc_imex(context, DC_IMEX_IMPORT_BACKUP, file, NULL);
 *         // connect
 *     }
 *     else
 *     {
 *         do {
 *             ask_user_for_credentials();
 *         }
 *         while (!configure_succeeded())
 *     }
 *     dc_str_unref(file);
 * }
 * ~~~
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param dir Directory to search backups in.
 * @return String with the backup file, typically given to dc_imex(),
 *     returned strings must be released using dc_str_unref().
 *     The function returns NULL if no backup was found.
 */
char*           dc_imex_has_backup           (dc_context_t* context, const char* dir);


/**
 * Initiate Autocrypt Setup Transfer.
 * Before starting the setup transfer with this function, the user should be asked:
 *
 * ~~~
 * "An 'Autocrypt Setup Message' securely shares your end-to-end setup with other Autocrypt-compliant apps.
 * The setup will be encrypted by a setup code which is displayed here and must be typed on the other device.
 * ~~~
 *
 * After that, this function should be called to send the Autocrypt Setup Message.
 * The function creates the setup message and waits until it is really sent.
 * As this may take a while, it is recommended to start the function in a separate thread;
 * to interrupt it, you can use dc_stop_ongoing_process().
 *
 * After everything succeeded, the required setup code is returned in the following format:
 *
 * ~~~
 * 1234-1234-1234-1234-1234-1234-1234-1234-1234
 * ~~~
 *
 * The setup code should be shown to the user then:
 *
 * ~~~
 * "Your key has been sent to yourself. Switch to the other device and
 * open the setup message. You should be prompted for a setup code. Type
 * the following digits into the prompt:
 *
 * 1234 - 1234 - 1234 -
 * 1234 - 1234 - 1234 -
 * 1234 - 1234 - 1234
 *
 * Once you're done, your other device will be ready to use Autocrypt."
 * ~~~
 *
 * On the _other device_ you will call dc_continue_key_transfer() then
 * for setup messages identified by dc_msg_is_setupmessage().
 *
 * For more details about the Autocrypt setup process, please refer to
 * https://autocrypt.org/en/latest/level1.html#autocrypt-setup-message
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @return The setup code. Must be released using dc_str_unref() after usage.
 *     On errors, eg. if the message could not be sent, NULL is returned.
 */
char*           dc_initiate_key_transfer     (dc_context_t* context);


/**
 * Continue the Autocrypt Key Transfer on another device.
 *
 * If you have started the key transfer on another device using dc_initiate_key_transfer()
 * and you've detected a setup message with dc_msg_is_setupmessage(), you should prompt the
 * user for the setup code and call this function then.
 *
 * You can use dc_msg_get_setupcodebegin() to give the user a hint about the code (useful if the user
 * has created several messages and should not enter the wrong code).
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param msg_id ID of the setup message to decrypt.
 * @param setup_code Setup code entered by the user. This is the same setup code as returned from
 *     dc_initiate_key_transfer() on the other device.
 *     There is no need to format the string correctly, the function will remove all spaces and other characters and
 *     insert the `-` characters at the correct places.
 * @return 1=key successfully decrypted and imported; both devices will use the same key now;
 *     0=key transfer failed eg. due to a bad setup code.
 */
int             dc_continue_key_transfer     (dc_context_t* context, uint32_t msg_id, const char* setup_code);


/**
 * Signal an ongoing process to stop.
 *
 * After that, dc_stop_ongoing_process() returns _without_ waiting
 * for the ongoing process to return.
 *
 * The ongoing process will return ASAP then, however, it may
 * still take a moment.
 *
 * Typical ongoing processes are started by dc_configure(),
 * dc_initiate_key_transfer() or dc_imex(). As there is always at most only
 * one onging process at the same time, there is no need to define _which_ process to exit.
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @return None.
 */
void            dc_stop_ongoing_process      (dc_context_t* context);


// out-of-band verification

#define         DC_QR_ASK_VERIFYCONTACT      200 // id=contact
#define         DC_QR_ASK_VERIFYGROUP        202 // text1=groupname
#define         DC_QR_FPR_OK                 210 // id=contact
#define         DC_QR_FPR_MISMATCH           220 // id=contact
#define         DC_QR_FPR_WITHOUT_ADDR       230 // test1=formatted fingerprint
#define         DC_QR_ACCOUNT                250 // text1=domain
#define         DC_QR_WEBRTC_INSTANCE        260 // text1=domain
#define         DC_QR_ADDR                   320 // id=contact
#define         DC_QR_TEXT                   330 // text1=text
#define         DC_QR_URL                    332 // text1=URL
#define         DC_QR_ERROR                  400 // text1=error string

/**
 * Check a scanned QR code.
 * The function should be called after a QR code is scanned.
 * The function takes the raw text scanned and checks what can be done with it.
 *
 * The QR code state is returned in dc_lot_t::state as:
 *
 * - DC_QR_ASK_VERIFYCONTACT with dc_lot_t::id=Contact ID
 * - DC_QR_ASK_VERIFYGROUP withdc_lot_t::text1=Group name
 * - DC_QR_FPR_OK with dc_lot_t::id=Contact ID
 * - DC_QR_FPR_MISMATCH with dc_lot_t::id=Contact ID
 * - DC_QR_FPR_WITHOUT_ADDR with dc_lot_t::test1=Formatted fingerprint
 * - DC_QR_ACCOUNT allows creation of an account, dc_lot_t::text1=domain
 * - DC_QR_WEBRTC_INSTANCE - a shared webrtc-instance
 *   that will be set if dc_set_config_from_qr() is called with the qr-code,
 *   dc_lot_t::text1=domain could be used to ask the user
 * - DC_QR_ADDR with dc_lot_t::id=Contact ID
 * - DC_QR_TEXT with dc_lot_t::text1=Text
 * - DC_QR_URL with dc_lot_t::text1=URL
 * - DC_QR_ERROR with dc_lot_t::text1=Error string
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param qr The text of the scanned QR code.
 * @return Parsed QR code as an dc_lot_t object. The returned object must be
 *     freed using dc_lot_unref() after usage.
 */
dc_lot_t*       dc_check_qr                  (dc_context_t* context, const char* qr);


/**
 * Get QR code text that will offer an Setup-Contact or Verified-Group invitation.
 * The QR code is compatible to the OPENPGP4FPR format
 * so that a basic fingerprint comparison also works eg. with OpenKeychain.
 *
 * The scanning device will pass the scanned content to dc_check_qr() then;
 * if dc_check_qr() returns DC_QR_ASK_VERIFYCONTACT or DC_QR_ASK_VERIFYGROUP
 * an out-of-band-verification can be joined using dc_join_securejoin()
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param chat_id If set to a group-chat-id,
 *     the Verified-Group-Invite protocol is offered in the QR code;
 *     works for verified groups as well as for normal groups.
 *     If set to 0, the Setup-Contact protocol is offered in the QR code.
 *     See https://countermitm.readthedocs.io/en/latest/new.html
 *     for details about both protocols.
 * @return Text that should go to the QR code,
 *     On errors, an empty QR code is returned, NULL is never returned.
 *     The returned string must be released using dc_str_unref() after usage.
 */
char*           dc_get_securejoin_qr         (dc_context_t* context, uint32_t chat_id);


/**
 * Continue a Setup-Contact or Verified-Group-Invite protocol
 * started on another device with dc_get_securejoin_qr().
 * This function is typically called when dc_check_qr() returns
 * lot.state=DC_QR_ASK_VERIFYCONTACT or lot.state=DC_QR_ASK_VERIFYGROUP.
 *
 * Depending on the given QR code,
 * this function may takes some time and sends and receives several messages.
 * Therefore, you should call it always in a separate thread;
 * if you want to abort it, you should call dc_stop_ongoing_process().
 *
 * - If the given QR code starts the Setup-Contact protocol,
 *   the function typically returns immediately
 *   and the handshake runs in background.
 *   Subsequent calls of dc_join_securejoin() will abort unfinished tasks.
 *   The returned chat is the one-to-one opportunistic chat.
 *   When the protocol has finished, an info-message is added to that chat.
 * - If the given QR code starts the Verified-Group-Invite protocol,
 *   the function waits until the protocol has finished.
 *   This is because the verified group is not opportunistic
 *   and can be created only when the contacts have verified each other.
 *
 * See https://countermitm.readthedocs.io/en/latest/new.html
 * for details about both protocols.
 *
 * @memberof dc_context_t
 * @param context The context object
 * @param qr The text of the scanned QR code. Typically, the same string as given
 *     to dc_check_qr().
 * @return Chat-id of the joined chat, the UI may redirect to the this chat.
 *     If the out-of-band verification failed or was aborted, 0 is returned.
 *     A returned chat-id does not guarantee that the chat or the belonging contact is verified.
 *     If needed, this be checked with dc_chat_is_verified() and dc_contact_is_verified(),
 *     however, in practise, the UI will just listen to #DC_EVENT_CONTACTS_CHANGED unconditionally.
 */
uint32_t        dc_join_securejoin           (dc_context_t* context, const char* qr);


// location streaming


/**
 * Enable or disable location streaming for a chat.
 * Locations are sent to all members of the chat for the given number of seconds;
 * after that, location streaming is automatically disabled for the chat.
 * The current location streaming state of a chat
 * can be checked using dc_is_sending_locations_to_chat().
 *
 * The locations that should be sent to the chat can be set using
 * dc_set_location().
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param chat_id Chat id to enable location streaming for.
 * @param seconds >0: enable location streaming for the given number of seconds;
 *     0: disable location streaming.
 * @return None.
 */
void        dc_send_locations_to_chat       (dc_context_t* context, uint32_t chat_id, int seconds);


/**
 * Check if location streaming is enabled.
 * Location stream can be enabled or disabled using dc_send_locations_to_chat().
 * If you have already a dc_chat_t object,
 * dc_chat_is_sending_locations() may be more handy.
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param chat_id >0: Check if location streaming is enabled for the given chat.
 *     0: Check of location streaming is enabled for any chat.
 * @return 1: location streaming is enabled for the given chat(s);
 *     0: location streaming is disabled for the given chat(s).
 */
int         dc_is_sending_locations_to_chat (dc_context_t* context, uint32_t chat_id);


/**
 * Set current location.
 * The location is sent to all chats where location streaming is enabled
 * using dc_send_locations_to_chat().
 *
 * Typically results in the event #DC_EVENT_LOCATION_CHANGED with
 * contact_id set to DC_CONTACT_ID_SELF.
 *
 * The UI should call this function on all location changes.
 * The locations set by this function are not sent immediately,
 * instead a message with the last locations is sent out every some minutes
 * or when the user sends out a normal message,
 * the last locations are attached.
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param latitude North-south position of the location.
 *     Set to 0.0 if the latitude is not known.
 * @param longitude East-west position of the location.
 *     Set to 0.0 if the longitude is not known.
 * @param accuracy Estimated accuracy of the location, radial, in meters.
 *     Set to 0.0 if the accuracy is not known.
 * @return 1: location streaming is still enabled for at least one chat,
 *     this dc_set_location() should be called as soon as the location changes;
 *     0: location streaming is no longer needed,
 *     dc_is_sending_locations_to_chat() is false for all chats.
 */
int         dc_set_location                 (dc_context_t* context, double latitude, double longitude, double accuracy);


/**
 * Get shared locations from the database.
 * The locations can be filtered by the chat-id, the contact-id
 * and by a timespan.
 *
 * The number of returned locations can be retrieved using dc_array_get_cnt().
 * To get information for each location,
 * use dc_array_get_latitude(), dc_array_get_longitude(),
 * dc_array_get_accuracy(), dc_array_get_timestamp(), dc_array_get_contact_id()
 * and dc_array_get_msg_id().
 * The latter returns 0 if there is no message bound to the location.
 *
 * Note that only if dc_array_is_independent() returns 0,
 * the location is the current or a past position of the user.
 * If dc_array_is_independent() returns 1,
 * the location is any location on earth that is marked by the user.
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param chat_id Chat-id to get location information for.
 *     0 to get locations independently of the chat.
 * @param contact_id Contact-id to get location information for.
 *     If also a chat-id is given, this should be a member of the given chat.
 *     0 to get locations independently of the contact.
 * @param timestamp_begin Start of timespan to return.
 *     Must be given in number of seconds since 00:00 hours, Jan 1, 1970 UTC.
 *     0 for "start from the beginning".
 * @param timestamp_end End of timespan to return.
 *     Must be given in number of seconds since 00:00 hours, Jan 1, 1970 UTC.
 *     0 for "all up to now".
 * @return Array of locations, NULL is never returned.
 *     The array is sorted decending;
 *     the first entry in the array is the location with the newest timestamp.
 *     Note that this is only realated to the recent postion of the user
 *     if dc_array_is_independent() returns 0.
 *     The returned array must be freed using dc_array_unref().
 *
 * Examples:
 * ~~~
 * // get locations from the last hour for a global map
 * dc_array_t* loc = dc_get_locations(context, 0, 0, time(NULL)-60*60, 0);
 * for (int i=0; i<dc_array_get_cnt(); i++) {
 *     double lat = dc_array_get_latitude(loc, i);
 *     ...
 * }
 * dc_array_unref(loc);
 *
 * // get locations from a contact for a global map
 * dc_array_t* loc = dc_get_locations(context, 0, contact_id, 0, 0);
 * ...
 *
 * // get all locations known for a given chat
 * dc_array_t* loc = dc_get_locations(context, chat_id, 0, 0, 0);
 * ...
 *
 * // get locations from a single contact for a given chat
 * dc_array_t* loc = dc_get_locations(context, chat_id, contact_id, 0, 0);
 * ...
 * ~~~
 */
dc_array_t* dc_get_locations                (dc_context_t* context, uint32_t chat_id, uint32_t contact_id, int64_t timestamp_begin, int64_t timestamp_end);


/**
 * Delete all locations on the current device.
 * Locations already sent cannot be deleted.
 *
 * Typically results in the event #DC_EVENT_LOCATION_CHANGED
 * with contact_id set to 0.
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @return None.
 */
void        dc_delete_all_locations         (dc_context_t* context);


/**
 * Release a string returned by another deltachat-core function.
 * - Strings returned by any deltachat-core-function
 *   MUST NOT be released by the standard free() function;
 *   always use dc_str_unref() for this purpose.
 * - dc_str_unref() MUST NOT be called for strings not returned by deltachat-core.
 * - dc_str_unref() MUST NOT be called for other objectes returned by deltachat-core.
 *
 * @memberof dc_context_t
 * @param str The string to release.
 * @return None.
 */
void dc_str_unref (char* str);


/**
 * @class dc_array_t
 *
 * An object containing a simple array.
 * This object is used in several places where functions need to return an array.
 * The items of the array are typically IDs.
 * To free an array object, use dc_array_unref().
 */


/**
 * Free an array object. Does not free any data items.
 *
 * @memberof dc_array_t
 * @param array The array object to free,
 *     created eg. by dc_get_chatlist(), dc_get_contacts() and so on.
 *     If NULL is given, nothing is done.
 * @return None.
 */
void             dc_array_unref              (dc_array_t* array);


/**
 * Find out the number of items in an array.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @return Returns the number of items in a dc_array_t object. 0 on errors or if the array is empty.
 */
size_t           dc_array_get_cnt            (const dc_array_t* array);


/**
 * Get the item at the given index as an ID.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item to get. Must be between 0 and dc_array_get_cnt()-1.
 * @return Returns the item at the given index. Returns 0 on errors or if the array is empty.
 */
uint32_t         dc_array_get_id             (const dc_array_t* array, size_t index);


/**
 * Return the latitude of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Latitude of the item at the given index.
 *     0.0 if there is no latitude bound to the given item,
 */
double           dc_array_get_latitude       (const dc_array_t* array, size_t index);


/**
 * Return the longitude of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Latitude of the item at the given index.
 *     0.0 if there is no longitude bound to the given item,
 */
double           dc_array_get_longitude      (const dc_array_t* array, size_t index);


/**
 * Return the accuracy of the item at the given index.
 * See dc_set_location() for more information about the accuracy.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Accuracy of the item at the given index.
 *     0.0 if there is no longitude bound to the given item,
 */
double           dc_array_get_accuracy       (const dc_array_t* array, size_t index);


/**
 * Return the timestamp of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Timestamp of the item at the given index.
 *     0 if there is no timestamp bound to the given item,
 */
int64_t           dc_array_get_timestamp      (const dc_array_t* array, size_t index);


/**
 * Return the chat-id of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Chat-id of the item at the given index.
 *     0 if there is no chat-id bound to the given item,
 */
uint32_t         dc_array_get_chat_id        (const dc_array_t* array, size_t index);


/**
 * Return the contact-id of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Contact-id of the item at the given index.
 *     0 if there is no contact-id bound to the given item,
 */
uint32_t         dc_array_get_contact_id     (const dc_array_t* array, size_t index);


/**
 * Return the message-id of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Message-id of the item at the given index.
 *     0 if there is no message-id bound to the given item,
 */
uint32_t         dc_array_get_msg_id         (const dc_array_t* array, size_t index);


/**
 * Return the marker-character of the item at the given index.
 * Marker-character are typically bound to locations
 * returned by dc_get_locations()
 * and are typically created by on-character-messages
 * which can also be an emoticon :)
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Marker-character of the item at the given index.
 *     NULL if there is no marker-character bound to the given item.
 *     The returned value must be released using dc_str_unref() after usage.
 */
char*            dc_array_get_marker         (const dc_array_t* array, size_t index);


/**
 * Return the independent-state of the location at the given index.
 * Independent locations do not belong to the track of the user.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return 0=Location belongs to the track of the user,
 *     1=Location was reported independently.
 */
int              dc_array_is_independent     (const dc_array_t* array, size_t index);


/**
 * Check if a given ID is present in an array.
 *
 * @private @memberof dc_array_t
 * @param array The array object to search in.
 * @param needle The ID to search for.
 * @param[out] ret_index If set, this will receive the index. Set to NULL if you're not interested in the index.
 * @return 1=ID is present in array, 0=ID not found.
 */
int              dc_array_search_id          (const dc_array_t* array, uint32_t needle, size_t* ret_index);


/**
 * @class dc_chatlist_t
 *
 * An object representing a single chatlist in memory.
 * Chatlist objects contain chat IDs
 * and, if possible, message IDs belonging to them.
 * The chatlist object is not updated;
 * if you want an update, you have to recreate the object.
 *
 * For a **typical chat overview**,
 * the idea is to get the list of all chats via dc_get_chatlist()
 * without any listflags (see below)
 * and to implement a "virtual list" or so
 * (the count of chats is known by dc_chatlist_get_cnt()).
 *
 * Only for the items that are in view
 * (the list may have several hundreds chats),
 * the UI should call dc_chatlist_get_summary() then.
 * dc_chatlist_get_summary() provides all elements needed for painting the item.
 *
 * On a click of such an item,
 * the UI should change to the chat view
 * and get all messages from this view via dc_get_chat_msgs().
 * Again, a "virtual list" is created
 * (the count of messages is known)
 * and for each messages that is scrolled into view, dc_get_msg() is called then.
 *
 * Why no listflags?
 * Without listflags, dc_get_chatlist() adds the deaddrop
 * and the archive "link" automatically as needed.
 * The UI can just render these items differently then.
 * Although the deaddrop link is currently always the first entry
 * and only present on new messages,
 * there is the rough idea that it can be optionally always present
 * and sorted into the list by date.
 * Rendering the deaddrop in the described way
 * would not add extra work in the UI then.
 */


/**
 * Free a chatlist object.
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist object to free, created eg. by dc_get_chatlist(), dc_search_msgs().
 *     If NULL is given, nothing is done.
 * @return None.
 */
void             dc_chatlist_unref           (dc_chatlist_t* chatlist);


/**
 * Find out the number of chats in a chatlist.
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist object as created eg. by dc_get_chatlist().
 * @return Returns the number of items in a dc_chatlist_t object. 0 on errors or if the list is empty.
 */
size_t           dc_chatlist_get_cnt         (const dc_chatlist_t* chatlist);


/**
 * Get a single chat ID of a chatlist.
 *
 * To get the message object from the message ID, use dc_get_chat().
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist object as created eg. by dc_get_chatlist().
 * @param index The index to get the chat ID for.
 * @return Returns the chat_id of the item at the given index.  Index must be between
 *     0 and dc_chatlist_get_cnt()-1.
 */
uint32_t         dc_chatlist_get_chat_id     (const dc_chatlist_t* chatlist, size_t index);


/**
 * Get a single message ID of a chatlist.
 *
 * To get the message object from the message ID, use dc_get_msg().
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist object as created eg. by dc_get_chatlist().
 * @param index The index to get the chat ID for.
 * @return Returns the message_id of the item at the given index.  Index must be between
 *     0 and dc_chatlist_get_cnt()-1.  If there is no message at the given index (eg. the chat may be empty), 0 is returned.
 */
uint32_t         dc_chatlist_get_msg_id      (const dc_chatlist_t* chatlist, size_t index);


/**
 * Get a summary for a chatlist index.
 *
 * The summary is returned by a dc_lot_t object with the following fields:
 *
 * - dc_lot_t::text1: contains the username or the strings "Me", "Draft" and so on.
 *   The string may be colored by having a look at text1_meaning.
 *   If there is no such name or it should not be displayed, the element is NULL.
 *
 * - dc_lot_t::text1_meaning: one of DC_TEXT1_USERNAME, DC_TEXT1_SELF or DC_TEXT1_DRAFT.
 *   Typically used to show dc_lot_t::text1 with different colors. 0 if not applicable.
 *
 * - dc_lot_t::text2: contains an excerpt of the message text or strings as
 *   "No messages".  May be NULL of there is no such text (eg. for the archive link)
 *
 * - dc_lot_t::timestamp: the timestamp of the message.  0 if not applicable.
 *
 * - dc_lot_t::state: The state of the message as one of the DC_STATE_* constants (see #dc_msg_get_state()).  0 if not applicable.
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist to query as returned eg. from dc_get_chatlist().
 * @param index The index to query in the chatlist.
 * @param chat To speed up things, pass an already available chat object here.
 *     If the chat object is not yet available, it is faster to pass NULL.
 * @return The summary as an dc_lot_t object. Must be freed using dc_lot_unref().  NULL is never returned.
 */
dc_lot_t*        dc_chatlist_get_summary     (const dc_chatlist_t* chatlist, size_t index, dc_chat_t* chat);


/**
 * Create a chatlist summary item when the chatlist object is already unref()'d.
 *
 * This function is similar to dc_chatlist_get_summary(), however,
 * takes the chat-id and message-id as returned by dc_chatlist_get_chat_id() and dc_chatlist_get_msg_id()
 * as arguments. The chatlist object itself is not needed directly.
 *
 * This maybe useful if you convert the complete object into a different represenation
 * as done eg. in the node-bindings.
 * If you have access to the chatlist object in some way, using this function is not recommended,
 * use dc_chatlist_get_summary() in this case instead.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new()
 * @param chat_id Chat to get a summary for.
 * @param msg_id Messasge to get a summary for.
 * @return The summary as an dc_lot_t object, see dc_chatlist_get_summary() for details.
 *     Must be freed using dc_lot_unref().  NULL is never returned.
 */
dc_lot_t*        dc_chatlist_get_summary2    (dc_context_t* context, uint32_t chat_id, uint32_t msg_id);


/**
 * Helper function to get the associated context object.
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist object to empty.
 * @return Context object associated with the chatlist. NULL if none or on errors.
 */
dc_context_t*    dc_chatlist_get_context     (dc_chatlist_t* chatlist);


/**
 * Get info summary for a chat, in json format.
 *
 * The returned json string has the following key/values:
 *
 * id: chat id
 * name: chat/group name
 * color: color of this chat
 * last-message-from: who sent the last message
 * last-message-text: message (truncated)
 * last-message-state: DC_STATE* constant
 * last-message-date:
 * avatar-path: path-to-blobfile
 * is_verified: yes/no
 * @return a utf8-encoded json string containing all requested info. Must be freed using dc_str_unref().  NULL is never returned.
 */
char*            dc_chat_get_info_json       (dc_context_t* context, size_t chat_id);

/**
 * @class dc_chat_t
 *
 * An object representing a single chat in memory.
 * Chat objects are created using eg. dc_get_chat()
 * and are not updated on database changes;
 * if you want an update, you have to recreate the object.
 */


#define         DC_CHAT_ID_DEADDROP          1 // virtual chat showing all messages belonging to chats flagged with chats.blocked=2
#define         DC_CHAT_ID_TRASH             3 // messages that should be deleted get this chat_id; the messages are deleted from the working thread later then. This is also needed as rfc724_mid should be preset as long as the message is not deleted on the server (otherwise it is downloaded again)
#define         DC_CHAT_ID_MSGS_IN_CREATION  4 // a message is just in creation but not yet assigned to a chat (eg. we may need the message ID to set up blobs; this avoids unready message to be sent and shown)
#define         DC_CHAT_ID_STARRED           5 // virtual chat showing all messages flagged with msgs.starred=2
#define         DC_CHAT_ID_ARCHIVED_LINK     6 // only an indicator in a chatlist
#define         DC_CHAT_ID_ALLDONE_HINT      7 // only an indicator in a chatlist
#define         DC_CHAT_ID_LAST_SPECIAL      9 // larger chat IDs are "real" chats, their messages are "real" messages.


#define         DC_CHAT_TYPE_UNDEFINED       0
#define         DC_CHAT_TYPE_SINGLE          100
#define         DC_CHAT_TYPE_GROUP           120
#define         DC_CHAT_TYPE_VERIFIED_GROUP  130


/**
 * Free a chat object.
 *
 * @memberof dc_chat_t
 * @param chat Chat object are returned eg. by dc_get_chat().
 *     If NULL is given, nothing is done.
 * @return None.
 */
void            dc_chat_unref                (dc_chat_t* chat);


/**
 * Get chat ID. The chat ID is the ID under which the chat is filed in the database.
 *
 * Special IDs:
 * - DC_CHAT_ID_DEADDROP         (1) - Virtual chat containing messages which senders are not confirmed by the user.
 * - DC_CHAT_ID_STARRED          (5) - Virtual chat containing all starred messages-
 * - DC_CHAT_ID_ARCHIVED_LINK    (6) - A link at the end of the chatlist, if present the UI should show the button "Archived chats"-
 *
 * "Normal" chat IDs are larger than these special IDs (larger than DC_CHAT_ID_LAST_SPECIAL).
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return Chat ID. 0 on errors.
 */
uint32_t        dc_chat_get_id               (const dc_chat_t* chat);


/**
 * Get chat type.
 *
 * Currently, there are two chat types:
 *
 * - DC_CHAT_TYPE_SINGLE (100) - a normal chat is a chat with a single contact,
 *   chats_contacts contains one record for the user.  DC_CONTACT_ID_SELF
 *   (see dc_contact_t::id) is added _only_ for a self talk.
 *
 * - DC_CHAT_TYPE_GROUP  (120) - a group chat, chats_contacts contain all group
 *   members, incl. DC_CONTACT_ID_SELF
 *
 * - DC_CHAT_TYPE_VERIFIED_GROUP  (130) - a verified group chat. In verified groups,
 *   all members are verified and encryption is always active and cannot be disabled.
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return Chat type.
 */
int             dc_chat_get_type             (const dc_chat_t* chat);


/**
 * Get name of a chat. For one-to-one chats, this is the name of the contact.
 * For group chats, this is the name given eg. to dc_create_group_chat() or
 * received by a group-creation message.
 *
 * To change the name, use dc_set_chat_name()
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return Chat name as a string. Must be released using dc_str_unref() after usage. Never NULL.
 */
char*           dc_chat_get_name             (const dc_chat_t* chat);


/**
 * Get the chat's profile image.
 * For groups, this is the image set by any group member
 * using dc_set_chat_profile_image().
 * For normal chats, this is the image set by each remote user on their own
 * using dc_set_config(context, "selfavatar", image).
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return Path and file if the profile image, if any.
 *     NULL otherwise.
 *     Must be released using dc_str_unref() after usage.
 */
char*           dc_chat_get_profile_image    (const dc_chat_t* chat);


/**
 * Get a color for the chat.
 * For 1:1 chats, the color is calculated from the contact's email address.
 * Otherwise, the chat name is used.
 * The color can be used for an fallback avatar with white initials
 * as well as for headlines in bubbles of group chats.
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return Color as 0x00rrggbb with rr=red, gg=green, bb=blue
 *     each in the range 0-255.
 */
uint32_t        dc_chat_get_color            (const dc_chat_t* chat);


/**
 * Get visibility of chat.
 * See @ref DC_CHAT_VISIBILITY for detailed information about the visibilities.
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return One of @ref DC_CHAT_VISIBILITY
 */
int             dc_chat_get_visibility       (const dc_chat_t* chat);


/**
 * Check if a group chat is still unpromoted.
 *
 * After the creation with dc_create_group_chat() the chat is usually unpromoted
 * until the first call to dc_send_text_msg() or another sending function.
 *
 * With unpromoted chats, members can be added
 * and settings can be modified without the need of special status messages being sent.
 *
 * While the core takes care of the unpromoted state on its own,
 * checking the state from the UI side may be useful to decide whether a hint as
 * "Send the first message to allow others to reply within the group"
 * should be shown to the user or not.
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 1=chat is still unpromoted, no message was ever send to the chat,
 *     0=chat is not unpromoted, messages were send and/or received
 *     or the chat is not group chat.
 */
int             dc_chat_is_unpromoted        (const dc_chat_t* chat);


/**
 * Check if a chat is a self talk.  Self talks are normal chats with
 * the only contact DC_CONTACT_ID_SELF.
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 1=chat is self talk, 0=chat is no self talk
 */
int             dc_chat_is_self_talk         (const dc_chat_t* chat);


/**
 * Check if a chat is a device-talk.
 * Device-talks contain update information
 * and some hints that are added during the program runs, multi-device etc.
 *
 * From the ui view, device-talks are not very special,
 * the user can delete and forward messages, archive the chat, set notifications etc.
 *
 * Messages can be added to the device-talk using dc_add_device_msg()
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 1=chat is device-talk, 0=chat is no device-talk
 */
int             dc_chat_is_device_talk       (const dc_chat_t* chat);


/**
 * Check if messages can be sent to a give chat.
 * This is not true eg. for the deaddrop or for the device-talk, cmp. dc_chat_is_device_talk().
 *
 * Calling dc_send_msg() for these chats will fail
 * and the ui may decide to hide input controls therefore.
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 1=chat is writable, 0=chat is not writable
 */
int             dc_chat_can_send              (const dc_chat_t* chat);


/**
 * Check if a chat is verified.  Verified chats contain only verified members
 * and encryption is alwasy enabled.  Verified chats are created using
 * dc_create_group_chat() by setting the 'verified' parameter to true.
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 1=chat verified, 0=chat is not verified
 */
int             dc_chat_is_verified          (const dc_chat_t* chat);


/**
 * Check if locations are sent to the chat
 * at the time the object was created using dc_get_chat().
 * To check if locations are sent to _any_ chat,
 * use dc_is_sending_locations_to_chat().
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 1=locations are sent to chat, 0=no locations are sent to chat
 */
int             dc_chat_is_sending_locations (const dc_chat_t* chat);


/**
 * Check whether the chat is currently muted (can be changed by dc_set_chat_mute_duration()).
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 1=muted, 0=not muted
 */
int             dc_chat_is_muted (const dc_chat_t* chat);


/**
 * Get the exact state of the mute of a chat
 *
 * @memberof dc_chat_t
 * @param chat The chat object.
 * @return 0=not muted, -1=forever muted, (x>0)=remaining seconds until the mute is lifted
 */
int64_t          dc_chat_get_remaining_mute_duration (const dc_chat_t* chat);


/**
 * @class dc_msg_t
 *
 * An object representing a single message in memory.
 * The message object is not updated.
 * If you want an update, you have to recreate the object.
 */


#define         DC_MSG_ID_MARKER1            1
#define         DC_MSG_ID_DAYMARKER          9
#define         DC_MSG_ID_LAST_SPECIAL       9


#define         DC_STATE_UNDEFINED           0
#define         DC_STATE_IN_FRESH            10
#define         DC_STATE_IN_NOTICED          13
#define         DC_STATE_IN_SEEN             16
#define         DC_STATE_OUT_PREPARING       18
#define         DC_STATE_OUT_DRAFT           19
#define         DC_STATE_OUT_PENDING         20
#define         DC_STATE_OUT_FAILED          24
#define         DC_STATE_OUT_DELIVERED       26 // to check if a mail was sent, use dc_msg_is_sent()
#define         DC_STATE_OUT_MDN_RCVD        28


#define         DC_MAX_GET_TEXT_LEN          30000 // approx. max. length returned by dc_msg_get_text()
#define         DC_MAX_GET_INFO_LEN          100000 // approx. max. length returned by dc_get_msg_info()


/**
 * Create new message object. Message objects are needed eg. for sending messages using
 * dc_send_msg().  Moreover, they are returned eg. from dc_get_msg(),
 * set up with the current state of a message. The message object is not updated;
 * to achieve this, you have to recreate it.
 *
 * @memberof dc_msg_t
 * @param context The context that should be stored in the message object.
 * @param viewtype The type to the message object to create,
 *     one of the @ref DC_MSG constants.
 * @return The created message object.
 */
dc_msg_t*       dc_msg_new                    (dc_context_t* context, int viewtype);


/**
 * Free a message object. Message objects are created eg. by dc_get_msg().
 *
 * @memberof dc_msg_t
 * @param msg The message object to free.
 *     If NULL is given, nothing is done.
 * @return None.
 */
void            dc_msg_unref                  (dc_msg_t* msg);


/**
 * Get the ID of the message.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return The ID of the message.
 *     0 if the given message object is invalid.
 */
uint32_t        dc_msg_get_id                 (const dc_msg_t* msg);


/**
 * Get the ID of contact who wrote the message.
 *
 * If the ID is equal to DC_CONTACT_ID_SELF (1), the message is an outgoing
 * message that is typically shown on the right side of the chat view.
 *
 * Otherwise, the message is an incoming message; to get details about the sender,
 * pass the returned ID to dc_get_contact().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return The ID of the contact who wrote the message, DC_CONTACT_ID_SELF (1)
 *     if this is an outgoing message, 0 on errors.
 */
uint32_t        dc_msg_get_from_id            (const dc_msg_t* msg);


/**
 * Get the ID of chat the message belongs to.
 * To get details about the chat, pass the returned ID to dc_get_chat().
 * If a message is still in the deaddrop, the ID DC_CHAT_ID_DEADDROP is returned
 * although internally another ID is used.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return The ID of the chat the message belongs to, 0 on errors.
 */
uint32_t        dc_msg_get_chat_id            (const dc_msg_t* msg);


/**
 * Get the type of the message.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return One of the @ref DC_MSG constants.
 *     0 if the given message object is invalid.
 */
int             dc_msg_get_viewtype           (const dc_msg_t* msg);


/**
 * Get the state of a message.
 *
 * Incoming message states:
 * - DC_STATE_IN_FRESH (10) - Incoming _fresh_ message. Fresh messages are neither noticed nor seen and are typically shown in notifications. Use dc_get_fresh_msgs() to get all fresh messages.
 * - DC_STATE_IN_NOTICED (13) - Incoming _noticed_ message. E.g. chat opened but message not yet read - noticed messages are not counted as unread but did not marked as read nor resulted in MDNs. Use dc_marknoticed_chat() or dc_marknoticed_contact() to mark messages as being noticed.
 * - DC_STATE_IN_SEEN (16) - Incoming message, really _seen_ by the user. Marked as read on IMAP and MDN may be sent. Use dc_markseen_msgs() to mark messages as being seen.
 *
 * Outgoing message states:
 * - DC_STATE_OUT_PREPARING (18) - For files which need time to be prepared before they can be sent,
 *   the message enters this state before DC_STATE_OUT_PENDING.
 * - DC_STATE_OUT_DRAFT (19) - Message saved as draft using dc_set_draft()
 * - DC_STATE_OUT_PENDING (20) - The user has pressed the "send" button but the
 *   message is not yet sent and is pending in some way. Maybe we're offline (no checkmark).
 * - DC_STATE_OUT_FAILED (24) - _Unrecoverable_ error (_recoverable_ errors result in pending messages), you'll receive the event #DC_EVENT_MSG_FAILED.
 * - DC_STATE_OUT_DELIVERED (26) - Outgoing message successfully delivered to server (one checkmark). Note, that already delivered messages may get into the state DC_STATE_OUT_FAILED if we get such a hint from the server.
 *   If a sent message changes to this state, you'll receive the event #DC_EVENT_MSG_DELIVERED.
 * - DC_STATE_OUT_MDN_RCVD (28) - Outgoing message read by the recipient (two checkmarks; this requires goodwill on the receiver's side)
 *   If a sent message changes to this state, you'll receive the event #DC_EVENT_MSG_READ.
 *   Also messages already read by some recipients
 *   may get into the state DC_STATE_OUT_FAILED at a later point,
 *   eg. when in a group, delivery fails for some recipients.
 *
 * If you just want to check if a message is sent or not, please use dc_msg_is_sent() which regards all states accordingly.
 *
 * The state of just created message objects is DC_STATE_UNDEFINED (0).
 * The state is always set by the core-library, users of the library cannot set the state directly, but it is changed implicitly eg.
 * when calling  dc_marknoticed_chat() or dc_markseen_msgs().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return The state of the message.
 */
int             dc_msg_get_state              (const dc_msg_t* msg);


/**
 * Get message sending time.
 * The sending time is returned as a unix timestamp in seconds.
 *
 * Note that the message lists returned eg. by dc_get_chat_msgs()
 * are not sorted by the _sending_ time but by the _receiving_ time.
 * This ensures newly received messages always pop up at the end of the list,
 * however, for delayed messages, the correct sending time will be displayed.
 *
 * To display detailed information about the times to the user,
 * the UI can use dc_get_msg_info().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return The time of the message.
 */
int64_t          dc_msg_get_timestamp          (const dc_msg_t* msg);


/**
 * Get message receive time.
 * The receive time is returned as a unix timestamp in seconds.
 *
 * To get the sending time, use dc_msg_get_timestamp().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Receiving time of the message.
 *     For outgoing messages, 0 is returned.
 */
int64_t          dc_msg_get_received_timestamp (const dc_msg_t* msg);


/**
 * Get message time used for sorting.
 * This function returns the timestamp that is used for sorting the message
 * into lists as returned eg. by dc_get_chat_msgs().
 * This may be the reveived time, the sending time or another time.
 *
 * To get the receiving time, use dc_msg_get_received_timestamp().
 * To get the sending time, use dc_msg_get_timestamp().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Time used for ordering.
 */
int64_t          dc_msg_get_sort_timestamp     (const dc_msg_t* msg);


/**
 * Get the text of the message.
 * If there is no text associated with the message, an empty string is returned.
 * NULL is never returned.
 *
 * The returned text is plain text, HTML is stripped.
 * The returned text is truncated to a max. length of currently about 30000 characters,
 * it does not make sense to show more text in the message list and typical controls
 * will have problems with showing much more text.
 * This max. length is to avoid passing _lots_ of data to the frontend which may
 * result eg. from decoding errors (assume some bytes missing in a mime structure, forcing
 * an attachment to be plain text).
 *
 * To get information about the message and more/raw text, use dc_get_msg_info().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Message text. The result must be released using dc_str_unref(). Never returns NULL.
 */
char*           dc_msg_get_text               (const dc_msg_t* msg);


/**
 * Find out full path, file name and extension of the file associated with a
 * message.
 *
 * Typically files are associated with images, videos, audios, documents.
 * Plain text messages do not have a file.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Full path, file name and extension of the file associated with the message.
 *     If there is no file associated with the message, an emtpy string is returned.
 *     NULL is never returned and the returned value must be released using dc_str_unref().
 */
char*           dc_msg_get_file               (const dc_msg_t* msg);


/**
 * Get base file name without path. The base file name includes the extension; the path
 * is not returned. To get the full path, use dc_msg_get_file().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Base file name plus extension without part.  If there is no file
 *     associated with the message, an empty string is returned.  The returned
 *     value must be released using dc_str_unref().
 */
char*           dc_msg_get_filename           (const dc_msg_t* msg);


/**
 * Get mime type of the file.  If there is not file, an empty string is returned.
 * If there is no associated mime type with the file, the function guesses on; if
 * in doubt, `application/octet-stream` is returned. NULL is never returned.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return String containing the mime type.
 *     Must be released using dc_str_unref() after usage. NULL is never returned.
 */
char*           dc_msg_get_filemime           (const dc_msg_t* msg);


/**
 * Get the size of the file.  Returns the size of the file associated with a
 * message, if applicable.
 *
 * Typically, this is used to show the size of document messages, eg. a PDF.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return File size in bytes, 0 if not applicable or on errors.
 */
uint64_t        dc_msg_get_filebytes          (const dc_msg_t* msg);


/**
 * Get width of image or video.  The width is returned in pixels.
 * If the width is unknown or if the associated file is no image or video file,
 * 0 is returned.
 *
 * Often the aspect ratio is the more interesting thing. You can calculate
 * this using dc_msg_get_width() / dc_msg_get_height().
 *
 * See also dc_msg_get_duration().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Width in pixels, if applicable. 0 otherwise or if unknown.
 */
int             dc_msg_get_width              (const dc_msg_t* msg);


/**
 * Get height of image or video.  The height is returned in pixels.
 * If the height is unknown or if the associated file is no image or video file,
 * 0 is returned.
 *
 * Often the ascpect ratio is the more interesting thing. You can calculate
 * this using dc_msg_get_width() / dc_msg_get_height().
 *
 * See also dc_msg_get_duration().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Height in pixels, if applicable. 0 otherwise or if unknown.
 */
int             dc_msg_get_height             (const dc_msg_t* msg);


/**
 * Get the duration of audio or video.  The duration is returned in milliseconds (ms).
 * If the duration is unknown or if the associated file is no audio or video file,
 * 0 is returned.
 *
 * See also dc_msg_get_width() and dc_msg_get_height().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Duration in milliseconds, if applicable. 0 otherwise or if unknown.
 */
int             dc_msg_get_duration           (const dc_msg_t* msg);


/**
 * Check if a padlock should be shown beside the message.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=padlock should be shown beside message, 0=do not show a padlock beside the message.
 */
int             dc_msg_get_showpadlock        (const dc_msg_t* msg);


/**
 * Get ephemeral timer duration for message.
 *
 * To check if the timer is started and calculate remaining time,
 * use dc_msg_get_ephemeral_timestamp().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Duration in seconds, or 0 if no timer is set.
 */
uint32_t        dc_msg_get_ephemeral_timer    (const dc_msg_t* msg);

/**
 * Get timestamp of ephemeral message removal.
 *
 * If returned value is non-zero, you can calculate the * fraction of
 * time remaining by divinding the difference between the current timestamp
 * and this timestamp by dc_msg_get_ephemeral_timer().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Time of message removal, 0 if the timer is not started.
 */
int64_t          dc_msg_get_ephemeral_timestamp (const dc_msg_t* msg);


/**
 * Get a summary for a message.
 *
 * The summary is returned by a dc_lot_t object with the following fields:
 *
 * - dc_lot_t::text1: contains the username or the string "Me".
 *   The string may be colored by having a look at text1_meaning.
 *   If the name should not be displayed, the element is NULL.
 * - dc_lot_t::text1_meaning: one of DC_TEXT1_USERNAME or DC_TEXT1_SELF.
 *   Typically used to show dc_lot_t::text1 with different colors. 0 if not applicable.
 * - dc_lot_t::text2: contains an excerpt of the message text.
 * - dc_lot_t::timestamp: the timestamp of the message.
 * - dc_lot_t::state: The state of the message as one of the DC_STATE_* constants (see #dc_msg_get_state()).
 *
 * Typically used to display a search result. See also dc_chatlist_get_summary() to display a list of chats.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param chat To speed up things, pass an already available chat object here.
 *     If the chat object is not yet available, it is faster to pass NULL.
 * @return The summary as an dc_lot_t object. Must be freed using dc_lot_unref().  NULL is never returned.
 */
dc_lot_t*       dc_msg_get_summary            (const dc_msg_t* msg, const dc_chat_t* chat);


/**
 * Get a message summary as a single line of text.  Typically used for
 * notifications.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param approx_characters Rough length of the expected string.
 * @return A summary for the given messages.
 *     The returned string must be released using dc_str_unref().
 *     Returns an empty string on errors, never returns NULL.
 */
char*           dc_msg_get_summarytext        (const dc_msg_t* msg, int approx_characters);


/**
 * Check if a message has a deviating timestamp.
 * A message has a deviating timestamp
 * when it is sent on another day as received/sorted by.
 *
 * When the UI displays normally only the time beside the message and the full day as headlines,
 * the UI should display the full date directly beside the message if the timestamp is deviating.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=Timestamp is deviating, the UI should display the full date beside the message.
 *     0=Timestamp is not deviating and belongs to the same date as the date headers,
 *     displaying the time only is sufficient in this case.
 */
int             dc_msg_has_deviating_timestamp(const dc_msg_t* msg);


/**
 * Check if a message has a location bound to it.
 * These messages are also returned by dc_get_locations()
 * and the UI may decide to display a special icon beside such messages,
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=Message has location bound to it, 0=No location bound to message.
 */
int             dc_msg_has_location           (const dc_msg_t* msg);


/**
 * Check if a message was sent successfully.
 *
 * Currently, "sent" messages are messages that are in the state "delivered" or "mdn received",
 * see dc_msg_get_state().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=message sent successfully, 0=message not yet sent or message is an incoming message.
 */
int             dc_msg_is_sent                (const dc_msg_t* msg);


/**
 * Check if a message is starred.  Starred messages are "favorites" marked by the user
 * with a "star" or something like that.  Starred messages can typically be shown
 * easily and are not deleted automatically.
 *
 * To star one or more messages, use dc_star_msgs(), to get a list of starred messages,
 * use dc_get_chat_msgs() using DC_CHAT_ID_STARRED as the chat_id.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=message is starred, 0=message not starred.
 */
int             dc_msg_is_starred             (const dc_msg_t* msg);


/**
 * Check if the message is a forwarded message.
 *
 * Forwarded messages may not be created by the contact given as "from".
 *
 * Typically, the UI shows a little text for a symbol above forwarded messages.
 *
 * For privacy reasons, we do not provide the name or the email address of the
 * original author (in a typical GUI, you select the messages text and click on
 * "forwared"; you won't expect other data to be send to the new recipient,
 * esp. as the new recipient may not be in any relationship to the original author)
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=message is a forwarded message, 0=message not forwarded.
 */
int             dc_msg_is_forwarded           (const dc_msg_t* msg);


/**
 * Check if the message is an informational message, created by the
 * device or by another users. Such messages are not "typed" by the user but
 * created due to other actions, eg. dc_set_chat_name(), dc_set_chat_profile_image()
 * or dc_add_contact_to_chat().
 *
 * These messages are typically shown in the center of the chat view,
 * dc_msg_get_text() returns a descriptive text about what is going on.
 *
 * There is no need to perform any action when seeing such a message - this is already done by the core.
 * Typically, these messages are displayed in the center of the chat.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=message is a system command, 0=normal message
 */
int             dc_msg_is_info                (const dc_msg_t* msg);


/**
 * Check if a message is still in creation.  A message is in creation between
 * the calls to dc_prepare_msg() and dc_send_msg().
 *
 * Typically, this is used for videos that are recoded by the UI before
 * they can be sent.
 *
 * @memberof dc_msg_t
 * @param msg The message object
 * @return 1=message is still in creation (dc_send_msg() was not called yet),
 *     0=message no longer in creation
 */
int             dc_msg_is_increation          (const dc_msg_t* msg);


/**
 * Check if the message is an Autocrypt Setup Message.
 *
 * Setup messages should be shown in an unique way eg. using a different text color.
 * On a click or another action, the user should be prompted for the setup code
 * which is forwarded to dc_continue_key_transfer() then.
 *
 * Setup message are typically generated by dc_initiate_key_transfer() on another device.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=message is a setup message, 0=no setup message.
 *     For setup messages, dc_msg_get_viewtype() returns DC_MSG_FILE.
 */
int             dc_msg_is_setupmessage        (const dc_msg_t* msg);


/**
 * Get the first characters of the setup code.
 *
 * Typically, this is used to pre-fill the first entry field of the setup code.
 * If the user has several setup messages, he can be sure typing in the correct digits.
 *
 * To check, if a message is a setup message, use dc_msg_is_setupmessage().
 * To decrypt a secret key from a setup message, use dc_continue_key_transfer().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Typically, the first two digits of the setup code or an empty string if unknown.
 *     NULL is never returned. Must be released using dc_str_unref() when done.
 */
char*           dc_msg_get_setupcodebegin     (const dc_msg_t* msg);


/**
 * Get url of a videochat invitation.
 *
 * Videochat invitations are sent out using dc_send_videochat_invitation()
 * and dc_msg_get_viewtype() returns #DC_MSG_VIDEOCHAT_INVITATION for such invitations.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return If the message contains a videochat invitation,
 *     the url of the invitation is returned.
 *     If the message is no videochat invitation, NULL is returned.
 *     Must be released using dc_str_unref() when done.
 */
char* dc_msg_get_videochat_url (const dc_msg_t* msg);


/**
 * Get type of videochat.
 *
 * Calling this functions only makes sense for messages of type #DC_MSG_VIDEOCHAT_INVITATION,
 * in this case, if "basic webrtc" as of https://github.com/cracker0dks/basicwebrtc was used to initiate the videochat,
 * dc_msg_get_videochat_type() returns DC_VIDEOCHATTYPE_BASICWEBRTC.
 * "basic webrtc" videochat may be processed natively by the app
 * whereas for other urls just the browser is opened.
 *
 * The videochat-url can be retrieved using dc_msg_get_videochat_url().
 * To check if a message is a videochat invitation at all, check the message type for #DC_MSG_VIDEOCHAT_INVITATION.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return Type of the videochat as of DC_VIDEOCHATTYPE_BASICWEBRTC or DC_VIDEOCHATTYPE_UNKNOWN.
 *
 * Example:
 * ~~~
 * if (dc_msg_get_viewtype(msg) == DC_MSG_VIDEOCHAT_INVITATION) {
 *   if (dc_msg_get_videochat_type(msg) == DC_VIDEOCHATTYPE_BASICWEBRTC) {
 *       // videochat invitation that we ship a client for
 *   } else {
 *       // use browser for videochat, just open the url
 *   }
 * } else {
 *    // not a videochat invitation
 * }
 * ~~~
 */
int dc_msg_get_videochat_type (const dc_msg_t* msg);

#define DC_VIDEOCHATTYPE_UNKNOWN     0
#define DC_VIDEOCHATTYPE_BASICWEBRTC 1


/**
 * Set the text of a message object.
 * This does not alter any information in the database; this may be done by dc_send_msg() later.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param text Message text.
 * @return None.
 */
void            dc_msg_set_text               (dc_msg_t* msg, const char* text);


/**
 * Set the file associated with a message object.
 * This does not alter any information in the database
 * nor copy or move the file or checks if the file exist.
 * All this can be done with dc_send_msg() later.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param file If the message object is used in dc_send_msg() later,
 *     this must be the full path of the image file to send.
 * @param filemime Mime type of the file. NULL if you don't know or don't care.
 * @return None.
 */
void            dc_msg_set_file               (dc_msg_t* msg, const char* file, const char* filemime);


/**
 * Set the dimensions associated with message object.
 * Typically this is the width and the height of an image or video associated using dc_msg_set_file().
 * This does not alter any information in the database; this may be done by dc_send_msg() later.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param width Width in pixels, if known. 0 if you don't know or don't care.
 * @param height Height in pixels, if known. 0 if you don't know or don't care.
 * @return None.
 */
void            dc_msg_set_dimension          (dc_msg_t* msg, int width, int height);


/**
 * Set the duration associated with message object.
 * Typically this is the duration of an audio or video associated using dc_msg_set_file().
 * This does not alter any information in the database; this may be done by dc_send_msg() later.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param duration Length in milliseconds. 0 if you don't know or don't care.
 * @return None.
 */
void            dc_msg_set_duration           (dc_msg_t* msg, int duration);


/**
 * Set any location that should be bound to the message object.
 * The function is useful to add a marker to the map
 * at a position different from the self-location.
 * You should not call this function
 * if you want to bind the current self-location to a message;
 * this is done by dc_set_location() and dc_send_locations_to_chat().
 *
 * Typically results in the event #DC_EVENT_LOCATION_CHANGED with
 * contact_id set to DC_CONTACT_ID_SELF.
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param latitude North-south position of the location.
 * @param longitude East-west position of the location.
 * @return None.
 */
void            dc_msg_set_location           (dc_msg_t* msg, double latitude, double longitude);


/**
 * Late filing information to a message.
 * In contrast to the dc_msg_set_*() functions, this function really stores the information in the database.
 *
 * Sometimes, the core cannot find out the width, the height or the duration
 * of an image, an audio or a video.
 *
 * If, in these cases, the frontend can provide the information, it can save
 * them together with the message object for later usage.
 *
 * This function should only be used if dc_msg_get_width(), dc_msg_get_height() or dc_msg_get_duration()
 * do not provide the expected values.
 *
 * To get the stored values later, use dc_msg_get_width(), dc_msg_get_height() or dc_msg_get_duration().
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param width The new width to store in the message object. 0 if you do not want to change width and height.
 * @param height The new height to store in the message object. 0 if you do not want to change width and height.
 * @param duration The new duration to store in the message object. 0 if you do not want to change it.
 * @return None.
 */
void            dc_msg_latefiling_mediasize   (dc_msg_t* msg, int width, int height, int duration);


/**
 * @class dc_contact_t
 *
 * An object representing a single contact in memory.
 * The contact object is not updated.
 * If you want an update, you have to recreate the object.
 *
 * The library makes sure
 * only to use names _authorized_ by the contact in `To:` or `Cc:`.
 * _Given-names _as "Daddy" or "Honey" are not used there.
 * For this purpose, internally, two names are tracked -
 * authorized-name and given-name.
 * By default, these names are equal,
 * but functions working with contact names
 * (eg. dc_contact_get_name(), dc_contact_get_display_name(),
 * dc_contact_get_name_n_addr(), dc_contact_get_first_name(),
 * dc_create_contact() or dc_add_address_book())
 * only affect the given-name.
 */


#define         DC_CONTACT_ID_SELF           1
#define         DC_CONTACT_ID_INFO           2 // centered messages as "member added", used in all chats
#define         DC_CONTACT_ID_DEVICE         5 // messages "update info" in the device-chat
#define         DC_CONTACT_ID_LAST_SPECIAL   9


/**
 * Free a contact object.
 *
 * @memberof dc_contact_t
 * @param contact The contact object as created eg. by dc_get_contact().
 *     If NULL is given, nothing is done.
 * @return None.
 */
void            dc_contact_unref             (dc_contact_t* contact);


/**
 * Get the ID of the contact.
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return The ID of the contact, 0 on errors.
 */
uint32_t        dc_contact_get_id            (const dc_contact_t* contact);


/**
 * Get email address.  The email address is always set for a contact.
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return String with the email address,
 *     must be released using dc_str_unref(). Never returns NULL.
 */
char*           dc_contact_get_addr          (const dc_contact_t* contact);


/**
 * Get the contact name. This is the name as defined by the contact himself or
 * modified by the user.  May be an empty string.
 *
 * This name is typically used in a form where the user can edit the name of a contact.
 * To get a fine name to display in lists etc., use dc_contact_get_display_name() or dc_contact_get_name_n_addr().
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return String with the name to display, must be released using dc_str_unref().
 *     Empty string if unset, never returns NULL.
 */
char*           dc_contact_get_name          (const dc_contact_t* contact);


/**
 * Get display name. This is the name as defined by the contact himself,
 * modified by the user or, if both are unset, the email address.
 *
 * This name is typically used in lists.
 * To get the name editable in a formular, use dc_contact_get_name().
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return String with the name to display, must be released using dc_str_unref().
 *     Never returns NULL.
 */
char*           dc_contact_get_display_name  (const dc_contact_t* contact);


/**
 * Get a summary of name and address.
 *
 * The returned string is either "Name (email@domain.com)" or just
 * "email@domain.com" if the name is unset.
 *
 * The summary is typically used when asking the user something about the contact.
 * The attached email address makes the question unique, eg. "Chat with Alan Miller (am@uniquedomain.com)?"
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return Summary string, must be released using dc_str_unref().
 *     Never returns NULL.
 */
char*           dc_contact_get_name_n_addr   (const dc_contact_t* contact);


/**
 * Get the part of the name before the first space. In most languages, this seems to be
 * the prename. If there is no space, the full display name is returned.
 * If the display name is not set, the e-mail address is returned.
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return String with the name to display, must be released using dc_str_unref().
 *     Never returns NULL.
 */
char*           dc_contact_get_first_name    (const dc_contact_t* contact);


/**
 * Get the contact's profile image.
 * This is the image set by each remote user on their own
 * using dc_set_config(context, "selfavatar", image).
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return Path and file if the profile image, if any.
 *     NULL otherwise.
 *     Must be released using dc_str_unref() after usage.
 */
char*           dc_contact_get_profile_image (const dc_contact_t* contact);


/**
 * Get a color for the contact.
 * The color is calculated from the contact's email address
 * and can be used for an fallback avatar with white initials
 * as well as for headlines in bubbles of group chats.
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return Color as 0x00rrggbb with rr=red, gg=green, bb=blue
 *     each in the range 0-255.
 */
uint32_t        dc_contact_get_color         (const dc_contact_t* contact);


/**
 * Check if a contact is blocked.
 *
 * To block or unblock a contact, use dc_block_contact().
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return 1=contact is blocked, 0=contact is not blocked.
 */
int             dc_contact_is_blocked        (const dc_contact_t* contact);


/**
 * Check if a contact was verified. E.g. by a secure-join QR code scan
 * and if the key has not changed since this verification.
 *
 * The UI may draw a checkbox or something like that beside verified contacts.
 *
 * @memberof dc_contact_t
 * @param contact The contact object.
 * @return 0: contact is not verified.
 *    2: SELF and contact have verified their fingerprints in both directions; in the UI typically checkmarks are shown.
 */
int             dc_contact_is_verified       (dc_contact_t* contact);


/**
 * @class dc_provider_t
 *
 * Opaque object containing information about one single email provider.
 */


/**
 * Create a provider struct for the given email address.
 *
 * The provider is extracted from the email address and it's information is returned.
 *
 * @memberof dc_provider_t
 * @param context The context object as created by dc_context_new().
 * @param email The user's email address to extract the provider info form.
 * @return a dc_provider_t struct which can be used with the dc_provider_get_*
 *     accessor functions.  If no provider info is found, NULL will be
 *     returned.
 */
dc_provider_t*  dc_provider_new_from_email            (const dc_context_t* context, const char* email);


/**
 * URL of the overview page.
 *
 * This URL allows linking to the providers page on providers.delta.chat.
 *
 * @memberof dc_provider_t
 * @param provider The dc_provider_t struct.
 * @return String with a fully-qualified URL,
 *     if there is no such URL, an empty string is returned, NULL is never returned.
 *     The returned value must be released using dc_str_unref().
 */
char*           dc_provider_get_overview_page         (const dc_provider_t* provider);


/**
 * Get hints to be shown to the user on the login screen.
 * Depending on the @ref DC_PROVIDER_STATUS returned by dc_provider_get_status(),
 * the ui may want to highlight the hint.
 *
 * Moreover, the ui should display a "More information" link
 * that forwards to the url returned by dc_provider_get_overview_page().
 *
 * @memberof dc_provider_t
 * @param provider The dc_provider_t struct.
 * @return A string with the hint to show to the user, may contain multiple lines,
 *     if there is no such hint, an empty string is returned, NULL is never returned.
 *     The returned value must be released using dc_str_unref().
 */
char*           dc_provider_get_before_login_hint     (const dc_provider_t* provider);


/**
 * Whether DC works with this provider.
 *
 * Can be one of #DC_PROVIDER_STATUS_OK,
 * #DC_PROVIDER_STATUS_PREPARATION or #DC_PROVIDER_STATUS_BROKEN.
 *
 * @memberof dc_provider_t
 * @param provider The dc_provider_t struct.
 * @return The status as a constant number.
 */
int             dc_provider_get_status                (const dc_provider_t* provider);


/**
 * Free the provider info struct.
 *
 * @memberof dc_provider_t
 * @param provider The dc_provider_t struct.
 */
void            dc_provider_unref                     (dc_provider_t* provider);


/**
 * @class dc_lot_t
 *
 * An object containing a set of values.
 * The meaning of the values is defined by the function returning the object.
 * Lot objects are created
 * eg. by dc_chatlist_get_summary() or dc_msg_get_summary().
 *
 * NB: _Lot_ is used in the meaning _heap_ here.
 */


#define         DC_TEXT1_DRAFT     1
#define         DC_TEXT1_USERNAME  2
#define         DC_TEXT1_SELF      3


/**
 * Frees an object containing a set of parameters.
 * If the set object contains strings, the strings are also freed with this function.
 * Set objects are created eg. by dc_chatlist_get_summary() or dc_msg_get_summary().
 *
 * @memberof dc_lot_t
 * @param lot The object to free.
 *     If NULL is given, nothing is done.
 * @return None.
 */
void            dc_lot_unref             (dc_lot_t* lot);


/**
 * Get first string. The meaning of the string is defined by the creator of the object and may be roughly described by dc_lot_get_text1_meaning().
 *
 * @memberof dc_lot_t
 * @param lot The lot object.
 * @return A string, the string may be empty
 *     and the returned value must be released using dc_str_unref().
 *     NULL if there is no such string.
 */
char*           dc_lot_get_text1         (const dc_lot_t* lot);


/**
 * Get second string. The meaning of the string is defined by the creator of the object.
 *
 * @memberof dc_lot_t
 * @param lot The lot object.
 * @return A string, the string may be empty
 *     and the returned value must be released using dc_str_unref().
 *     NULL if there is no such string.
 */
char*           dc_lot_get_text2         (const dc_lot_t* lot);


/**
 * Get the meaning of the first string.  Posssible meanings of the string are defined by the creator of the object and may be returned eg.
 * as DC_TEXT1_DRAFT, DC_TEXT1_USERNAME or DC_TEXT1_SELF.
 *
 * @memberof dc_lot_t
 * @param lot The lot object.
 * @return Returns the meaning of the first string, possible meanings are defined by the creator of the object.
 *    0 if there is no concrete meaning or on errors.
 */
int             dc_lot_get_text1_meaning (const dc_lot_t* lot);


/**
 * Get the associated state. The meaning of the state is defined by the creator of the object.
 *
 * @memberof dc_lot_t
 * @param lot The lot object.
 * @return The state as defined by the creator of the object. 0 if there is not state or on errors.
 */
int             dc_lot_get_state         (const dc_lot_t* lot);


/**
 * Get the associated ID. The meaning of the ID is defined by the creator of the object.
 *
 * @memberof dc_lot_t
 * @param lot The lot object.
 * @return The state as defined by the creator of the object. 0 if there is not state or on errors.
 */
uint32_t        dc_lot_get_id            (const dc_lot_t* lot);


/**
 * Get the associated timestamp.
 * The timestamp is returned as a unix timestamp in seconds.
 * The meaning of the timestamp is defined by the creator of the object.
 *
 * @memberof dc_lot_t
 * @param lot The lot object.
 * @return The timestamp as defined by the creator of the object. 0 if there is not timestamp or on errors.
 */
int64_t          dc_lot_get_timestamp     (const dc_lot_t* lot);


/**
 * @defgroup DC_MSG DC_MSG
 *
 * With these constants the type of a message is defined.
 *
 * From the view of the library,
 * all types are primary types of the same level,
 * eg. the library does not regard #DC_MSG_GIF as a subtype for #DC_MSG_IMAGE
 * and it's up to the UI to decide whether a GIF is shown
 * eg. in an IMAGE or in a VIDEO container.
 *
 * If you want to define the type of a dc_msg_t object for sending,
 * use dc_msg_new().
 *
 * To get the types of dc_msg_t objects received, use dc_msg_get_viewtype().
 *
 * @addtogroup DC_MSG
 * @{
 */


/**
 * Text message.
 * The text of the message is set using dc_msg_set_text()
 * and retrieved with dc_msg_get_text().
 */
#define DC_MSG_TEXT      10


/**
 * Image message.
 * If the image is an animated GIF, the type DC_MSG_GIF should be used.
 * File, width and height are set via dc_msg_set_file(), dc_msg_set_dimension
 * and retrieved via dc_msg_set_file(), dc_msg_set_dimension().
 */
#define DC_MSG_IMAGE     20


/**
 * Animated GIF message.
 * File, width and height are set via dc_msg_set_file(), dc_msg_set_dimension()
 * and retrieved via dc_msg_get_file(), dc_msg_get_width(), dc_msg_get_height().
 */
#define DC_MSG_GIF       21


/**
 * Message containing a sticker, similar to image.
 * If possible, the ui should display the image without borders in a transparent way.
 * A click on a sticker will offer to install the sticker set in some future.
 */
#define DC_MSG_STICKER     23


/**
 * Message containing an Audio file.
 * File and duration are set via dc_msg_set_file(), dc_msg_set_duration()
 * and retrieved via dc_msg_get_file(), dc_msg_get_duration().
 */
#define DC_MSG_AUDIO     40


/**
 * A voice message that was directly recorded by the user.
 * For all other audio messages, the type #DC_MSG_AUDIO should be used.
 * File and duration are set via dc_msg_set_file(), dc_msg_set_duration()
 * and retrieved via dc_msg_get_file(), dc_msg_get_duration()
 */
#define DC_MSG_VOICE     41


/**
 * Video messages.
 * File, width, height and durarion
 * are set via dc_msg_set_file(), dc_msg_set_dimension(), dc_msg_set_duration()
 * and retrieved via
 * dc_msg_get_file(), dc_msg_get_width(),
 * dc_msg_get_height(), dc_msg_get_duration().
 */
#define DC_MSG_VIDEO     50


/**
 * Message containing any file, eg. a PDF.
 * The file is set via dc_msg_set_file()
 * and retrieved via dc_msg_get_file().
 */
#define DC_MSG_FILE      60


/**
 * Message indicating an incoming or outgoing videochat.
 * The message was created via dc_send_videochat_invitation() on this or a remote device.
 *
 * Typically, such messages are rendered differently by the UIs,
 * eg. contain a button to join the videochat.
 * The url for joining can be retrieved using dc_msg_get_videochat_url().
 */
#define DC_MSG_VIDEOCHAT_INVITATION 70


/**
 * @}
 */


/**
 * @defgroup DC_LP DC_LP
 *
 * Flags for configuring IMAP and SMTP servers.
 * These flags are optional
 * and may be set together with the username, password etc.
 * via dc_set_config() using the key "server_flags".
 *
 * @addtogroup DC_LP
 * @{
 */


/**
 * Force OAuth2 authorization. This flag does not skip automatic configuration.
 * Before calling dc_configure() with DC_LP_AUTH_OAUTH2 set,
 * the user has to confirm access at the URL returned by dc_get_oauth2_url().
 */
#define DC_LP_AUTH_OAUTH2                0x2


/**
 * Force NORMAL authorization, this is the default.
 * If this flag is set, automatic configuration is skipped.
 */
#define DC_LP_AUTH_NORMAL                0x4


/**
 * Connect to IMAP via STARTTLS.
 * If this flag is set, automatic configuration is skipped.
 */
#define DC_LP_IMAP_SOCKET_STARTTLS     0x100


/**
 * Connect to IMAP via SSL.
 * If this flag is set, automatic configuration is skipped.
 */
#define DC_LP_IMAP_SOCKET_SSL          0x200


/**
 * Connect to IMAP unencrypted, this should not be used.
 * If this flag is set, automatic configuration is skipped.
 */
#define DC_LP_IMAP_SOCKET_PLAIN        0x400


/**
 * Connect to SMTP via STARTTLS.
 * If this flag is set, automatic configuration is skipped.
 */
#define DC_LP_SMTP_SOCKET_STARTTLS   0x10000


/**
 * Connect to SMTP via SSL.
 * If this flag is set, automatic configuration is skipped.
 */
#define DC_LP_SMTP_SOCKET_SSL        0x20000


/**
 * Connect to SMTP unencrypted, this should not be used.
 * If this flag is set, automatic configuration is skipped.
 */
#define DC_LP_SMTP_SOCKET_PLAIN      0x40000 ///<

/**
 * @}
 */

#define DC_LP_AUTH_FLAGS        (DC_LP_AUTH_OAUTH2|DC_LP_AUTH_NORMAL) // if none of these flags are set, the default is chosen
#define DC_LP_IMAP_SOCKET_FLAGS (DC_LP_IMAP_SOCKET_STARTTLS|DC_LP_IMAP_SOCKET_SSL|DC_LP_IMAP_SOCKET_PLAIN) // if none of these flags are set, the default is chosen
#define DC_LP_SMTP_SOCKET_FLAGS (DC_LP_SMTP_SOCKET_STARTTLS|DC_LP_SMTP_SOCKET_SSL|DC_LP_SMTP_SOCKET_PLAIN) // if none of these flags are set, the default is chosen

/**
 * @defgroup DC_CERTCK DC_CERTCK
 *
 * These constants configure TLS certificate checks for IMAP and SMTP connections.
 *
 * These constants are set via dc_set_config()
 * using keys "imap_certificate_checks" and "smtp_certificate_checks".
 *
 * @addtogroup DC_CERTCK
 * @{
 */

/**
 * Configure certificate checks automatically.
 */
#define DC_CERTCK_AUTO 0

/**
 * Strictly check TLS certificates;
 * require that both the certificate and hostname are valid.
 */
#define DC_CERTCK_STRICT 1

/**
 * Accept invalid certificates, including self-signed ones
 * or having incorrect hostname.
 */
#define DC_CERTCK_ACCEPT_INVALID_CERTIFICATES 3

/**
 * @}
 */


#define DC_EMPTY_MVBOX 0x01 // Deprecated, flag for dc_empty_server(): Clear all mvbox messages
#define DC_EMPTY_INBOX 0x02 // Deprecated, flag for dc_empty_server(): Clear all INBOX messages


/**
 * @class dc_event_emitter_t
 *
 * Opaque object that is used to get events.
 * You can get an event emitter from a context using dc_get_event_emitter().
 */

/**
 * Get the next event from an event emitter object.
 *
 * @memberof dc_event_emitter_t
 * @param emitter Event emitter object as returned from dc_get_event_emitter().
 * @return An event as an dc_event_t object.
 *     You can query the event for information using dc_event_get_id(), dc_event_get_data1_int() and so on;
 *     if you are done with the event, you have to free the event using dc_event_unref().
 *     If NULL is returned, the context belonging to the event emitter is unref'd and the no more events will come;
 *     in this case, free the event emitter using dc_event_emitter_unref().
 */
dc_event_t* dc_get_next_event(dc_event_emitter_t* emitter);


/**
 * Free an event emitter object.
 *
 * @memberof dc_event_emitter_t
 * @param emitter Event emitter object as returned from dc_get_event_emitter().
 *     If NULL is given, nothing is done and an error is logged.
 * @return None.
 */
void  dc_event_emitter_unref(dc_event_emitter_t* emitter);


/**
 * @class dc_event_t
 *
 * Opaque object describing a single event.
 * To get events, call dc_get_next_event() on an event emitter created by dc_get_event_emitter().
 */

/**
 * Get the event-id from an event object.
 * The event-id is one of the @ref DC_EVENT constants.
 * There may be additional data belonging to an event,
 * to get them, use dc_event_get_data1_int(), dc_event_get_data2_int() and dc_event_get_data2_str().
 *
 * @memberof dc_event_t
 * @param event Event object as returned from dc_get_next_event().
 * @return once of the @ref DC_EVENT constants.
 *     0 on errors.
 */
int dc_event_get_id(dc_event_t* event);


/**
 * Get a data associated with an event object.
 * The meaning of the data depends on the event-id
 * returned as @ref DC_EVENT constants by dc_event_get_id().
 * See also dc_event_get_data2_int() and dc_event_get_data2_str().
 *
 * @memberof dc_event_t
 * @param event Event object as returned from dc_get_next_event().
 * @return "data1" as a signed integer, at least 32bit,
 *     the meaning depends on the event type associated with this event.
 */
int dc_event_get_data1_int(dc_event_t* event);


/**
 * Get a data associated with an event object.
 * The meaning of the data depends on the event-id
 * returned as @ref DC_EVENT constants by dc_event_get_id().
 * See also dc_event_get_data2_int() and dc_event_get_data2_str().
 *
 * @memberof dc_event_t
 * @param event Event object as returned from dc_get_next_event().
 * @return "data2" as a signed integer, at least 32bit,
 *     the meaning depends on the event type associated with this event.
 */
int dc_event_get_data2_int(dc_event_t* event);


/**
 * Get a data associated with an event object.
 * The meaning of the data depends on the event-id
 * returned as @ref DC_EVENT constants by dc_event_get_id().
 * See also dc_event_get_data1_int() and dc_event_get_data2_int().
 *
 * @memberof dc_event_t
 * @param event Event object as returned from dc_get_next_event().
 * @return "data2" as a string,
 *     the meaning depends on the event type associated with this event.
 *     Once you're done with the string, you have to unref it using dc_unref_str().
 */
char* dc_event_get_data2_str(dc_event_t* event);


/**
 * Free memory used by an event object.
 * If you forget to do this for an event, this will result in memory leakage.
 *
 * @memberof dc_event_t
 * @param event Event object as returned from dc_get_next_event().
 * @return None.
 */
void dc_event_unref(dc_event_t* event);


/**
 * @defgroup DC_EVENT DC_EVENT
 *
 * These constants are used as event-id
 * in events returned by dc_get_next_event().
 *
 * Events typically come with some additional data,
 * use dc_event_get_data1_int(), dc_event_get_data2_int() and dc_event_get_data2_str() to read this data.
 * The meaning of the data depends on the event.
 *
 * @addtogroup DC_EVENT
 * @{
 */

/**
 * The library-user may write an informational string to the log.
 *
 * This event should not be reported to the end-user using a popup or something like that.
 *
 * @param data1 0
 * @param data2 (char*) Info string in english language.
 */
#define DC_EVENT_INFO                     100


/**
 * Emitted when SMTP connection is established and login was successful.
 *
 * @param data1 0
 * @param data2 (char*) Info string in english language.
 */
#define DC_EVENT_SMTP_CONNECTED           101


/**
 * Emitted when IMAP connection is established and login was successful.
 *
 * @param data1 0
 * @param data2 (char*) Info string in english language.
 */
#define DC_EVENT_IMAP_CONNECTED           102

/**
 * Emitted when a message was successfully sent to the SMTP server.
 *
 * @param data1 0
 * @param data2 (char*) Info string in english language.
 */
#define DC_EVENT_SMTP_MESSAGE_SENT        103

/**
 * Emitted when a message was successfully marked as deleted on the IMAP server.
 *
 * @param data1 0
 * @param data2 (char*) Info string in english language.
 */
#define DC_EVENT_IMAP_MESSAGE_DELETED   104

/**
 * Emitted when a message was successfully moved on IMAP.
 *
 * @param data1 0
 * @param data2 (char*) Info string in english language.
 */
#define DC_EVENT_IMAP_MESSAGE_MOVED   105

/**
 * Emitted when an IMAP folder was emptied.
 *
 * @param data1 0
 * @param data2 (char*) Folder name.
 */
#define DC_EVENT_IMAP_FOLDER_EMPTIED  106

/**
 * Emitted when a new blob file was successfully written
 *
 * @param data1 0
 * @param data2 (char*) Path name
 */
#define DC_EVENT_NEW_BLOB_FILE 150

/**
 * Emitted when a blob file was successfully deleted
 *
 * @param data1 0
 * @param data2 (char*) Path name
 */
#define DC_EVENT_DELETED_BLOB_FILE 151

/**
 * The library-user should write a warning string to the log.
 *
 * This event should not be reported to the end-user using a popup or something like that.
 *
 * @param data1 0
 * @param data2 (char*) Warning string in english language.
 */
#define DC_EVENT_WARNING                  300


/**
 * The library-user should report an error to the end-user.
 *
 * As most things are asynchronous, things may go wrong at any time and the user
 * should not be disturbed by a dialog or so.  Instead, use a bubble or so.
 *
 * However, for ongoing processes (eg. dc_configure())
 * or for functions that are expected to fail (eg. dc_continue_key_transfer())
 * it might be better to delay showing these events until the function has really
 * failed (returned false). It should be sufficient to report only the _last_ error
 * in a messasge box then.
 *
 * @param data1 0
 * @param data2 (char*) Error string, always set, never NULL.
 *     Some error strings are taken from dc_set_stock_translation(),
 *     however, most error strings will be in english language.
 */
#define DC_EVENT_ERROR                    400


/**
 * An action cannot be performed because there is no network available.
 *
 * The library will typically try over after a some time
 * and when dc_maybe_network() is called.
 *
 * Network errors should be reported to users in a non-disturbing way,
 * however, as network errors may come in a sequence,
 * it is not useful to raise each an every error to the user.
 *
 * Moreover, if the UI detects that the device is offline,
 * it is probably more useful to report this to the user
 * instead of the string from data2.
 *
 * @param data1 0
 * @param data2 (char*) Error string, always set, never NULL.
 */
#define DC_EVENT_ERROR_NETWORK            401


/**
 * An action cannot be performed because the user is not in the group.
 * Reported eg. after a call to
 * dc_set_chat_name(), dc_set_chat_profile_image(),
 * dc_add_contact_to_chat(), dc_remove_contact_from_chat(),
 * dc_send_text_msg() or another sending function.
 *
 * @param data1 0
 * @param data2 (char*) Info string in english language.
 */
#define DC_EVENT_ERROR_SELF_NOT_IN_GROUP  410


/**
 * Messages or chats changed.  One or more messages or chats changed for various
 * reasons in the database:
 * - Messages sent, received or removed
 * - Chats created, deleted or archived
 * - A draft has been set
 *
 * @param data1 (int) chat_id for single added messages
 * @param data2 (int) msg_id for single added messages
 */
#define DC_EVENT_MSGS_CHANGED             2000


/**
 * There is a fresh message. Typically, the user will show an notification
 * when receiving this message.
 *
 * There is no extra #DC_EVENT_MSGS_CHANGED event send together with this event.
 *
 * @param data1 (int) chat_id
 * @param data2 (int) msg_id
 */
#define DC_EVENT_INCOMING_MSG             2005


/**
 * A single message is sent successfully. State changed from  DC_STATE_OUT_PENDING to
 * DC_STATE_OUT_DELIVERED, see dc_msg_get_state().
 *
 * @param data1 (int) chat_id
 * @param data2 (int) msg_id
 */
#define DC_EVENT_MSG_DELIVERED            2010


/**
 * A single message could not be sent.
 * State changed from DC_STATE_OUT_PENDING, DC_STATE_OUT_DELIVERED or DC_STATE_OUT_MDN_RCVD
 * to DC_STATE_OUT_FAILED, see dc_msg_get_state().
 *
 * @param data1 (int) chat_id
 * @param data2 (int) msg_id
 */
#define DC_EVENT_MSG_FAILED               2012


/**
 * A single message is read by the receiver. State changed from DC_STATE_OUT_DELIVERED to
 * DC_STATE_OUT_MDN_RCVD, see dc_msg_get_state().
 *
 * @param data1 (int) chat_id
 * @param data2 (int) msg_id
 */
#define DC_EVENT_MSG_READ                 2015


/**
 * Chat changed.  The name or the image of a chat group was changed or members were added or removed.
 * Or the verify state of a chat has changed.
 * See dc_set_chat_name(), dc_set_chat_profile_image(), dc_add_contact_to_chat()
 * and dc_remove_contact_from_chat().
 *
 * @param data1 (int) chat_id
 * @param data2 0
 */
#define DC_EVENT_CHAT_MODIFIED            2020

/**
 * Chat ephemeral timer changed.
 */
#define DC_EVENT_CHAT_EPHEMERAL_TIMER_MODIFIED 2021


/**
 * Contact(s) created, renamed, verified, blocked or deleted.
 *
 * @param data1 (int) If not 0, this is the contact_id of an added contact that should be selected.
 * @param data2 0
 */
#define DC_EVENT_CONTACTS_CHANGED         2030



/**
 * Location of one or more contact has changed.
 *
 * @param data1 (int) contact_id of the contact for which the location has changed.
 *     If the locations of several contacts have been changed,
 *     eg. after calling dc_delete_all_locations(), this parameter is set to 0.
 * @param data2 0
 */
#define DC_EVENT_LOCATION_CHANGED         2035


/**
 * Inform about the configuration progress started by dc_configure().
 *
 * @param data1 (int) 0=error, 1-999=progress in permille, 1000=success and done
 * @param data2 0
 */
#define DC_EVENT_CONFIGURE_PROGRESS       2041


/**
 * Inform about the import/export progress started by dc_imex().
 *
 * @param data1 (int) 0=error, 1-999=progress in permille, 1000=success and done
 * @param data2 0
 */
#define DC_EVENT_IMEX_PROGRESS            2051


/**
 * A file has been exported. A file has been written by dc_imex().
 * This event may be sent multiple times by a single call to dc_imex().
 *
 * A typical purpose for a handler of this event may be to make the file public to some system
 * services.
 *
 * @param data1 0
 * @param data2 (char*) Path and file name.
 */
#define DC_EVENT_IMEX_FILE_WRITTEN        2052


/**
 * Progress information of a secure-join handshake from the view of the inviter
 * (Alice, the person who shows the QR code).
 *
 * These events are typically sent after a joiner has scanned the QR code
 * generated by dc_get_securejoin_qr().
 *
 * @param data1 (int) ID of the contact that wants to join.
 * @param data2 (int) Progress as:
 *     300=vg-/vc-request received, typically shown as "bob@addr joins".
 *     600=vg-/vc-request-with-auth received, vg-member-added/vc-contact-confirm sent, typically shown as "bob@addr verified".
 *     800=vg-member-added-received received, shown as "bob@addr securely joined GROUP", only sent for the verified-group-protocol.
 *     1000=Protocol finished for this contact.
 */
#define DC_EVENT_SECUREJOIN_INVITER_PROGRESS      2060


/**
 * Progress information of a secure-join handshake from the view of the joiner
 * (Bob, the person who scans the QR code).
 *
 * The events are typically sent while dc_join_securejoin(), which
 * may take some time, is executed.
 *
 * @param data1 (int) ID of the inviting contact.
 * @param data2 (int) Progress as:
 *     400=vg-/vc-request-with-auth sent, typically shown as "alice@addr verified, introducing myself."
 *     (Bob has verified alice and waits until Alice does the same for him)
 */
#define DC_EVENT_SECUREJOIN_JOINER_PROGRESS       2061

/**
 * @}
 */


#define DC_EVENT_FILE_COPIED         2055 // not used anymore
#define DC_EVENT_IS_OFFLINE          2081 // not used anymore
#define DC_EVENT_GET_STRING          2091 // not used anymore, use dc_set_stock_translation()
#define DC_ERROR_SEE_STRING          0    // not used anymore
#define DC_ERROR_SELF_NOT_IN_GROUP   1    // not used anymore
#define DC_STR_SELFNOTINGRP          21   // not used anymore
#define DC_EVENT_DATA1_IS_STRING(e)  0    // not used anymore 
#define DC_EVENT_DATA2_IS_STRING(e)  ((e)==DC_EVENT_IMEX_FILE_WRITTEN || ((e)>=100 && (e)<=499))
#define DC_EVENT_RETURNS_INT(e)      ((e)==DC_EVENT_IS_OFFLINE) // not used anymore
#define DC_EVENT_RETURNS_STRING(e)   ((e)==DC_EVENT_GET_STRING) // not used anymore
#define dc_archive_chat(a,b,c)  dc_set_chat_visibility((a), (b), (c)? 1 : 0) // not used anymore
#define dc_chat_get_archived(a) (dc_chat_get_visibility((a))==1? 1 : 0)      // not used anymore


/*
 * Values for dc_get|set_config("show_emails")
 */
#define DC_SHOW_EMAILS_OFF               0
#define DC_SHOW_EMAILS_ACCEPTED_CONTACTS 1
#define DC_SHOW_EMAILS_ALL               2


/*
 * Values for dc_get|set_config("media_quality")
 */
#define DC_MEDIA_QUALITY_BALANCED 0
#define DC_MEDIA_QUALITY_WORSE    1


/*
 * Values for dc_get|set_config("key_gen_type")
 */
#define DC_KEY_GEN_DEFAULT 0
#define DC_KEY_GEN_RSA2048 1
#define DC_KEY_GEN_ED25519 2


/**
 * @defgroup DC_PROVIDER_STATUS DC_PROVIDER_STATUS
 *
 * These constants are used as return values for dc_provider_get_status().
 *
 * @addtogroup DC_PROVIDER_STATUS
 * @{
 */

/**
 * Provider works out-of-the-box.
 * This provider status is returned for provider where the login
 * works by just entering the name or the email-address.
 *
 * - There is no need for the user to do any special things
 *   (enable IMAP or so) in the provider's webinterface or at other places.
 * - There is no need for the user to enter advanced settings;
 *   server, port etc. are known by the core.
 *
 * The status is returned by dc_provider_get_status().
 */
#define         DC_PROVIDER_STATUS_OK           1

/**
 * Provider works, but there are preparations needed.
 *
 * - The user has to do some special things as "Enable IMAP in the Webinterface",
 *   what exactly, is described in the string returnd by dc_provider_get_before_login_hints()
 *   and, typically more detailed, in the page linked by dc_provider_get_overview_page().
 * - There is no need for the user to enter advanced settings;
 *   server, port etc. should be known by the core.
 *
 * The status is returned by dc_provider_get_status().
 */
#define         DC_PROVIDER_STATUS_PREPARATION  2

/**
 * Provider is not working.
 * This provider status is returned for providers
 * that are known to not work with Delta Chat.
 * The ui should block logging in with this provider.
 *
 * More information about that is typically provided
 * in the string returned by dc_provider_get_before_login_hints()
 * and in the page linked by dc_provider_get_overview_page().
 *
 * The status is returned by dc_provider_get_status().
 */
#define         DC_PROVIDER_STATUS_BROKEN       3

/**
 * @}
 */


/**
 * @defgroup DC_CHAT_VISIBILITY DC_CHAT_VISIBILITY
 *
 * These constants describe the visibility of a chat.
 * The chat visibiliry can be get using dc_chat_get_visibility()
 * and set using dc_set_chat_visibility().
 *
 * @addtogroup DC_CHAT_VISIBILITY
 * @{
 */

/**
 * Chats with normal visibility are not archived and are shown below all pinned chats.
 * Archived chats, that receive new messages automatically become normal chats.
 */
#define         DC_CHAT_VISIBILITY_NORMAL      0

/**
 * Archived chats are not included in the default chatlist returned by dc_get_chatlist().
 * Instead, if there are _any_ archived chats, the pseudo-chat
 * with the chat_id DC_CHAT_ID_ARCHIVED_LINK will be added the the end of the chatlist.
 *
 * The UI typically shows a little icon or chats beside archived chats in the chatlist,
 * this is needed as eg. the search will also return archived chats.
 *
 * If archived chats receive new messages, they become normal chats again.
 *
 * To get a list of archived chats, use dc_get_chatlist() with the flag DC_GCL_ARCHIVED_ONLY.
 */
#define         DC_CHAT_VISIBILITY_ARCHIVED    1

/**
 * Pinned chats are included in the default chatlist. moreover,
 * they are always the first items, whether they have fresh messages or not.
 */
#define         DC_CHAT_VISIBILITY_PINNED      2

/**
 * @}
 */


/*
 * TODO: Strings need some doumentation about used placeholders.
 *
 * @defgroup DC_STR DC_STR
 *
 * These constants are used to request strings using #DC_EVENT_GET_STRING.
 *
 * @addtogroup DC_STR
 * @{
 */
#define DC_STR_NOMESSAGES                 1
#define DC_STR_SELF                       2
#define DC_STR_DRAFT                      3
#define DC_STR_VOICEMESSAGE               7
#define DC_STR_DEADDROP                   8
#define DC_STR_IMAGE                      9
#define DC_STR_VIDEO                      10
#define DC_STR_AUDIO                      11
#define DC_STR_FILE                       12
#define DC_STR_STATUSLINE                 13
#define DC_STR_NEWGROUPDRAFT              14
#define DC_STR_MSGGRPNAME                 15
#define DC_STR_MSGGRPIMGCHANGED           16
#define DC_STR_MSGADDMEMBER               17
#define DC_STR_MSGDELMEMBER               18
#define DC_STR_MSGGROUPLEFT               19
#define DC_STR_GIF                        23
#define DC_STR_ENCRYPTEDMSG               24
#define DC_STR_E2E_AVAILABLE              25
#define DC_STR_ENCR_TRANSP                27
#define DC_STR_ENCR_NONE                  28
#define DC_STR_CANTDECRYPT_MSG_BODY       29
#define DC_STR_FINGERPRINTS               30
#define DC_STR_READRCPT                   31
#define DC_STR_READRCPT_MAILBODY          32
#define DC_STR_MSGGRPIMGDELETED           33
#define DC_STR_E2E_PREFERRED              34
#define DC_STR_CONTACT_VERIFIED           35
#define DC_STR_CONTACT_NOT_VERIFIED       36
#define DC_STR_CONTACT_SETUP_CHANGED      37
#define DC_STR_ARCHIVEDCHATS              40
#define DC_STR_STARREDMSGS                41
#define DC_STR_AC_SETUP_MSG_SUBJECT       42
#define DC_STR_AC_SETUP_MSG_BODY          43
#define DC_STR_CANNOT_LOGIN               60
#define DC_STR_SERVER_RESPONSE            61
#define DC_STR_MSGACTIONBYUSER            62
#define DC_STR_MSGACTIONBYME              63
#define DC_STR_MSGLOCATIONENABLED         64
#define DC_STR_MSGLOCATIONDISABLED        65
#define DC_STR_LOCATION                   66
#define DC_STR_STICKER                    67
#define DC_STR_DEVICE_MESSAGES            68
#define DC_STR_SAVED_MESSAGES             69
#define DC_STR_DEVICE_MESSAGES_HINT       70
#define DC_STR_WELCOME_MESSAGE            71
#define DC_STR_UNKNOWN_SENDER_FOR_CHAT    72
#define DC_STR_SUBJECT_FOR_NEW_CONTACT    73
#define DC_STR_FAILED_SENDING_TO          74
#define DC_STR_EPHEMERAL_DISABLED         75
#define DC_STR_EPHEMERAL_SECONDS          76
#define DC_STR_EPHEMERAL_MINUTE           77
#define DC_STR_EPHEMERAL_HOUR             78
#define DC_STR_EPHEMERAL_DAY              79
#define DC_STR_EPHEMERAL_WEEK             80
#define DC_STR_EPHEMERAL_FOUR_WEEKS       81
#define DC_STR_VIDEOCHAT_INVITATION       82
#define DC_STR_VIDEOCHAT_INVITE_MSG_BODY  83

#define DC_STR_COUNT                      83

/*
 * @}
 */

#ifdef PY_CFFI_INC
/* Helper utility to locate the header file when building python bindings. */
char* _dc_header_file_location(void) {
    return __FILE__;
}
#endif


#ifdef __cplusplus
}
#endif
#endif // __DELTACHAT_H__
