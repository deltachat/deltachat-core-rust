#ifndef __DELTACHAT_H__
#define __DELTACHAT_H__
#ifdef __cplusplus
extern "C" {
#endif


#ifndef PY_CFFI
#include <stdint.h>
#include <time.h>
#endif


/**
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


/**
 * @class dc_context_t
 *
 * An object representing a single account.
 *
 * Each account is linked to an IMAP/SMTP account and uses a separate
 * SQLite database for offline functionality and for account-related
 * settings.
 */
typedef struct _dc_context  dc_context_t;
typedef struct _dc_array    dc_array_t;
typedef struct _dc_chatlist dc_chatlist_t;
typedef struct _dc_chat     dc_chat_t;
typedef struct _dc_msg      dc_msg_t;
typedef struct _dc_contact  dc_contact_t;
typedef struct _dc_lot      dc_lot_t;


/**
 * Callback function that should be given to dc_context_new().
 *
 * @memberof dc_context_t
 * @param context The context object as returned by dc_context_new().
 * @param event one of the @ref DC_EVENT constants
 * @param data1 depends on the event parameter
 * @param data2 depends on the event parameter
 * @return return 0 unless stated otherwise in the event parameter documentation
 */
typedef uintptr_t (*dc_callback_t) (dc_context_t*, int event, uintptr_t data1, uintptr_t data2);


// create/open/config/information

/**
 * Create a new context object.  After creation it is usually
 * opened, connected and mails are fetched.
 *
 * @memberof dc_context_t
 * @param cb a callback function that is called for events (update,
 *     state changes etc.) and to get some information from the client (eg. translation
 *     for a given string).
 *     See @ref DC_EVENT for a list of possible events that may be passed to the callback.
 *     - The callback MAY be called from _any_ thread, not only the main/GUI thread!
 *     - The callback MUST NOT call any dc_* and related functions unless stated
 *       otherwise!
 *     - The callback SHOULD return _fast_, for GUI updates etc. you should
 *       post yourself an asynchronous message to your GUI thread, if needed.
 *     - If not mentioned otherweise, the callback should return 0.
 * @param userdata can be used by the client for any purpuse.  He finds it
 *     later in dc_get_userdata().
 * @param os_name is only for decorative use
 *     and is shown eg. in the `X-Mailer:` header
 *     in the form "Delta Chat Core <version>/<os_name>".
 *     You can give the name of the app, the operating system,
 *     the used environment and/or the version here.
 *     It is okay to give NULL, in this case `X-Mailer:` header
 *     is set to "Delta Chat Core <version>".
 * @return A context object with some public members.
 *     The object must be passed to the other context functions
 *     and must be freed using dc_context_unref() after usage.
 */
dc_context_t*   dc_context_new               (dc_callback_t, void* userdata, const char* os_name);


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
void            dc_context_unref             (dc_context_t*);


/**
 * Get user data associated with a context object.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return User data, this is the second parameter given to dc_context_new().
 */
void*           dc_get_userdata              (dc_context_t*);


/**
 * Open context database.  If the given file does not exist, it is
 * created and can be set up using dc_set_config() afterwards.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param dbfile The file to use to store the database, something like `~/file` won't
 *     work on all systems, if in doubt, use absolute paths.
 * @param blobdir A directory to store the blobs in; a trailing slash is not needed.
 *     If you pass NULL or the empty string, deltachat-core creates a directory
 *     beside _dbfile_ with the same name and the suffix `-blobs`.
 * @return 1 on success, 0 on failure
 *     eg. if the file is not writable
 *     or if there is already a database opened for the context.
 */
int             dc_open                      (dc_context_t*, const char* dbfile, const char* blobdir);


/**
 * Close context database opened by dc_open().
 * Before this, connections to SMTP and IMAP are closed; these connections
 * are started automatically as needed eg. by sending for fetching messages.
 * This function is also implicitly called by dc_context_unref().
 * Multiple calls to this functions are okay, the function takes care not
 * to free objects twice.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return None.
 */
void            dc_close                     (dc_context_t*);


/**
 * Check if the context database is open.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return 0=context is not open, 1=context is open.
 */
int             dc_is_open                   (const dc_context_t*);


/**
 * Get the blob directory.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return Blob directory associated with the context object, empty string if unset or on errors. NULL is never returned.
 *     The returned string must be free()'d.
 */
char*           dc_get_blobdir               (const dc_context_t*);


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
 * - `displayname`  = Own name to use when sending messages.  MUAs are allowed to spread this way eg. using CC, defaults to empty
 * - `selfstatus`   = Own status to display eg. in email footers, defaults to a standard text
 * - `selfavatar`   = File containing avatar. Will be copied to blob directory.
 *                    NULL to remove the avatar.
 *                    It is planned for future versions
 *                    to send this image together with the next messages.
 * - `e2ee_enabled` = 0=no end-to-end-encryption, 1=prefer end-to-end-encryption (default)
 * - `mdns_enabled` = 0=do not send or request read receipts,
 *                    1=send and request read receipts (default)
 * - `inbox_watch`  = 1=watch `INBOX`-folder for changes (default),
 *                    0=do not watch the `INBOX`-folder
 * - `sentbox_watch`= 1=watch `Sent`-folder for changes (default),
 *                    0=do not watch the `Sent`-folder
 * - `mvbox_watch`  = 1=watch `DeltaChat`-folder for changes (default),
 *                    0=do not watch the `DeltaChat`-folder
 * - `mvbox_move`   = 1=heuristically detect chat-messages
 *                    and move them to the `DeltaChat`-folder,
 *                    0=do not move chat-messages
 * - `show_emails`  = DC_SHOW_EMAILS_OFF (0)=
 *                    show direct replies to chats only (default),
 *                    DC_SHOW_EMAILS_ACCEPTED_CONTACTS (1)=
 *                    also show all mails of confirmed contacts,
 *                    DC_SHOW_EMAILS_ALL (2)=
 *                    also show mails of unconfirmed contacts in the deaddrop.
 * - `save_mime_headers` = 1=save mime headers
 *                    and make dc_get_mime_headers() work for subsequent calls,
 *                    0=do not save mime headers (default)
 *
 * If you want to retrieve a value, use dc_get_config().
 *
 * @memberof dc_context_t
 * @param context The context object
 * @param key The option to change, see above.
 * @param value The value to save for "key"
 * @return 0=failure, 1=success
 */
int             dc_set_config                (dc_context_t*, const char* key, const char* value);


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
 * @return Returns current value of "key", if "key" is unset, the default value is returned.
 *     The returned value must be free()'d, NULL is never returned.
 */
char*           dc_get_config                (dc_context_t*, const char* key);


/**
 * Get information about the context.
 * The information is returned by a multi-line string
 * and contains information about the current configuration.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return String which must be free()'d after usage.  Never returns NULL.
 */
char*           dc_get_info                  (dc_context_t*);


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
 * Note: OAuth2 depends on #DC_EVENT_HTTP_POST;
 * if you have not implemented #DC_EVENT_HTTP_POST in the ui,
 * OAuth2 **won't work**.
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
 *     If OAuth2 is not possible for the given e-mail-address, NULL is returned.
 */
char*           dc_get_oauth2_url            (dc_context_t*, const char* addr, const char* redirect);



// connect

/**
 * Configure a context.
 * For this purpose, the function creates a job
 * that is executed in the IMAP-thread then;
 * this requires to call dc_perform_imap_jobs() regularly.
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
 * On a successfull configuration,
 * the core makes a copy of the parameters mentioned above:
 * the original parameters as are never modified by the core.
 *
 * UI-implementors should keep this in mind -
 * eg. if the UI wants to prefill a configure-edit-dialog with these parameters,
 * the UI should reset them if the user cancels the dialog
 * after a configure-attempts has failed.
 * Otherwise the parameters may not reflect the current configuation.
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
void            dc_configure                 (dc_context_t*);


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
int             dc_is_configured             (const dc_context_t*);


/**
 * Execute pending imap-jobs.
 * This function and dc_perform_imap_fetch() and dc_perform_imap_idle()
 * must be called from the same thread, typically in a loop.
 *
 * Example:
 *
 *     void* imap_thread_func(void* context)
 *     {
 *         while (true) {
 *             dc_perform_imap_jobs(context);
 *             dc_perform_imap_fetch(context);
 *             dc_perform_imap_idle(context);
 *         }
 *     }
 *
 *     // start imap-thread that runs forever
 *     pthread_t imap_thread;
 *     pthread_create(&imap_thread, NULL, imap_thread_func, context);
 *
 *     ... program runs ...
 *
 *     // network becomes available again -
 *     // the interrupt causes dc_perform_imap_idle() in the thread above
 *     // to return so that jobs are executed and messages are fetched.
 *     dc_maybe_network(context);
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_imap_jobs         (dc_context_t*);


/**
 * Fetch new messages, if any.
 * This function and dc_perform_imap_jobs() and dc_perform_imap_idle() must be called from the same thread,
 * typically in a loop.
 *
 * See dc_perform_imap_jobs() for an example.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_imap_fetch        (dc_context_t*);


/**
 * Wait for messages or jobs.
 * This function and dc_perform_imap_jobs() and dc_perform_imap_fetch() must be called from the same thread,
 * typically in a loop.
 *
 * You should call this function directly after calling dc_perform_imap_fetch().
 *
 * See dc_perform_imap_jobs() for an example.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_imap_idle         (dc_context_t*);


/**
 * Interrupt waiting for imap-jobs.
 * If dc_perform_imap_jobs(), dc_perform_imap_fetch() and dc_perform_imap_idle() are called in a loop,
 * calling this function causes imap-jobs to be executed and messages to be fetched.
 *
 * dc_interrupt_imap_idle() does _not_ interrupt dc_perform_imap_jobs() or dc_perform_imap_fetch().
 * If the imap-thread is inside one of these functions when dc_interrupt_imap_idle() is called, however,
 * the next call of the imap-thread to dc_perform_imap_idle() is interrupted immediately.
 *
 * Internally, this function is called whenever a imap-jobs should be processed
 * (delete message, markseen etc.).
 *
 * When you need to call this function just because to get jobs done after network changes,
 * use dc_maybe_network() instead.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_interrupt_imap_idle       (dc_context_t*);


/**
 * Fetch new messages from the MVBOX, if any.
 * The MVBOX is a folder on the account where chat messages are moved to.
 * The moving is done to not disturb shared accounts that are used by both,
 * Delta Chat and a classical MUA.
 *
 * This function and dc_perform_mvbox_idle()
 * must be called from the same thread, typically in a loop.
 *
 * Example:
 *
 *     void* mvbox_thread_func(void* context)
 *     {
 *         while (true) {
 *             dc_perform_mvbox_fetch(context);
 *             dc_perform_mvbox_idle(context);
 *         }
 *     }
 *
 *     // start mvbox-thread that runs forever
 *     pthread_t mvbox_thread;
 *     pthread_create(&mvbox_thread, NULL, mvbox_thread_func, context);
 *
 *     ... program runs ...
 *
 *     // network becomes available again -
 *     // the interrupt causes dc_perform_mvbox_idle() in the thread above
 *     // to return so that and messages are fetched.
 *     dc_maybe_network(context);
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_mvbox_fetch       (dc_context_t*);


/**
 * Wait for messages or jobs in the MVBOX-thread.
 * This function and dc_perform_mvbox_fetch().
 * must be called from the same thread, typically in a loop.
 *
 * You should call this function directly after calling dc_perform_mvbox_fetch().
 *
 * See dc_perform_mvbox_fetch() for an example.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_mvbox_idle        (dc_context_t*);


/**
 * Interrupt waiting for MVBOX-fetch.
 * dc_interrupt_mvbox_idle() does _not_ interrupt dc_perform_mvbox_fetch().
 * If the MVBOX-thread is inside this function when dc_interrupt_mvbox_idle() is called, however,
 * the next call of the MVBOX-thread to dc_perform_mvbox_idle() is interrupted immediately.
 *
 * Internally, this function is called whenever a imap-jobs should be processed.
 *
 * When you need to call this function just because to get jobs done after network changes,
 * use dc_maybe_network() instead.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_interrupt_mvbox_idle      (dc_context_t*);


/**
 * Fetch new messages from the Sent folder, if any.
 * This function and dc_perform_sentbox_idle()
 * must be called from the same thread, typically in a loop.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_sentbox_fetch     (dc_context_t*);


/**
 * Wait for messages or jobs in the SENTBOX-thread.
 * This function and dc_perform_sentbox_fetch()
 * must be called from the same thread, typically in a loop.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_sentbox_idle      (dc_context_t*);


/**
 * Interrupt waiting for messages or jobs in the SENTBOX-thread.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_interrupt_sentbox_idle    (dc_context_t*);


/**
 * Execute pending smtp-jobs.
 * This function and dc_perform_smtp_idle() must be called from the same thread,
 * typically in a loop.
 *
 * Example:
 *
 *     void* smtp_thread_func(void* context)
 *     {
 *         while (true) {
 *             dc_perform_smtp_jobs(context);
 *             dc_perform_smtp_idle(context);
 *         }
 *     }
 *
 *     // start smtp-thread that runs forever
 *     pthread_t smtp_thread;
 *     pthread_create(&smtp_thread, NULL, smtp_thread_func, context);
 *
 *     ... program runs ...
 *
 *     // network becomes available again -
 *     // the interrupt causes dc_perform_smtp_idle() in the thread above
 *     // to return so that jobs are executed
 *     dc_maybe_network(context);
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_smtp_jobs         (dc_context_t*);


/**
 * Wait for smtp-jobs.
 * This function and dc_perform_smtp_jobs() must be called from the same thread,
 * typically in a loop.
 *
 * See dc_interrupt_smtp_idle() for an example.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_perform_smtp_idle         (dc_context_t*);


/**
 * Interrupt waiting for smtp-jobs.
 * If dc_perform_smtp_jobs() and dc_perform_smtp_idle() are called in a loop,
 * calling this function causes jobs to be executed.
 *
 * dc_interrupt_smtp_idle() does _not_ interrupt dc_perform_smtp_jobs().
 * If the smtp-thread is inside this function when dc_interrupt_smtp_idle() is called, however,
 * the next call of the smtp-thread to dc_perform_smtp_idle() is interrupted immediately.
 *
 * Internally, this function is called whenever a message is to be sent.
 *
 * When you need to call this function just because to get jobs done after network changes,
 * use dc_maybe_network() instead.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_interrupt_smtp_idle       (dc_context_t*);


/**
 * This function can be called whenever there is a hint
 * that the network is available again.
 * The library will try to send pending messages out.
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @return None.
 */
void            dc_maybe_network             (dc_context_t*);


// handle chatlists

#define         DC_GCL_ARCHIVED_ONLY         0x01
#define         DC_GCL_NO_SPECIALS           0x02
#define         DC_GCL_ADD_ALLDONE_HINT      0x04


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
 *   archived _any_ chat using dc_archive_chat(). The UI should show a link as
 *   "Show archived chats", if the user clicks this item, the UI should show a
 *   list of all archived chats that can be created by this function hen using
 *   the DC_GCL_ARCHIVED_ONLY flag.
 * - DC_CHAT_ID_ALLDONE_HINT (7) - this special chat is present
 *   if DC_GCL_ADD_ALLDONE_HINT is added to listflags
 *   and if there are only archived chats.
 *
 * @memberof dc_context_t
 * @param context The context object as returned by dc_context_new()
 * @param listflags A combination of flags:
 *     - if the flag DC_GCL_ARCHIVED_ONLY is set, only archived chats are returned.
 *       if DC_GCL_ARCHIVED_ONLY is not set, only unarchived chats are returned and
 *       the pseudo-chat DC_CHAT_ID_ARCHIVED_LINK is added if there are _any_ archived
 *       chats
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
dc_chatlist_t*  dc_get_chatlist              (dc_context_t*, int flags, const char* query_str, uint32_t query_id);


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
uint32_t        dc_create_chat_by_msg_id     (dc_context_t*, uint32_t msg_id);


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
uint32_t        dc_create_chat_by_contact_id (dc_context_t*, uint32_t contact_id);


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
uint32_t        dc_get_chat_id_by_contact_id (dc_context_t*, uint32_t contact_id);


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
 * dc_msg_t* msg = dc_msg_new(context, DC_MSG_VIDEO);
 * dc_msg_set_file(msg, "/file/to/send.mp4", NULL);
 * dc_prepare_msg(context, chat_id, msg);
 * // ... after /file/to/send.mp4 is ready:
 * dc_send_msg(context, chat_id, msg);
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
uint32_t        dc_prepare_msg               (dc_context_t*, uint32_t chat_id, dc_msg_t*);


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
 * dc_msg_set_file(msg, "/file/to/send.jpg", NULL);
 * dc_send_msg(context, chat_id, msg);
 * ~~~
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
uint32_t        dc_send_msg                  (dc_context_t*, uint32_t chat_id, dc_msg_t*);


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
uint32_t        dc_send_text_msg             (dc_context_t*, uint32_t chat_id, const char* text_to_send);


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
void            dc_set_draft                 (dc_context_t*, uint32_t chat_id, dc_msg_t*);


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
dc_msg_t*       dc_get_draft                 (dc_context_t*, uint32_t chat_id);


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
 * @param marker1before An optional message ID.  If set, the id DC_MSG_ID_MARKER1 will be added just
 *   before the given ID in the returned array.  Set this to 0 if you do not want this behaviour.
 * @return Array of message IDs, must be dc_array_unref()'d when no longer used.
 */
dc_array_t*     dc_get_chat_msgs             (dc_context_t*, uint32_t chat_id, uint32_t flags, uint32_t marker1before);


/**
 * Get the total number of messages in a chat.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to count the messages for.
 * @return Number of total messages in the given chat. 0 for errors or empty chats.
 */
int             dc_get_msg_cnt               (dc_context_t*, uint32_t chat_id);


/**
 * Get the number of _fresh_ messages in a chat.  Typically used to implement
 * a badge with a number in the chatlist.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to count the messages for.
 * @return Number of fresh messages in the given chat. 0 for errors or if there are no fresh messages.
 */
int             dc_get_fresh_msg_cnt         (dc_context_t*, uint32_t chat_id);


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
dc_array_t*     dc_get_fresh_msgs            (dc_context_t*);


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
void            dc_marknoticed_chat          (dc_context_t*, uint32_t chat_id);


/**
 * Same as dc_marknoticed_chat() but for _all_ chats.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @return None.
 */
void            dc_marknoticed_all_chats     (dc_context_t*);


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
 * @param msg_type Specify a message type to query here, one of the DC_MSG_* constats.
 * @param msg_type2 Alternative message type to search for. 0 to skip.
 * @param msg_type3 Alternative message type to search for. 0 to skip.
 * @return An array with messages from the given chat ID that have the wanted message types.
 */
dc_array_t*     dc_get_chat_media            (dc_context_t*, uint32_t chat_id, int msg_type, int or_msg_type2, int or_msg_type3);


/**
 * Search next/previous message based on a given message and a list of types.
 * The
 * Typically used to implement the "next" and "previous" buttons
 * in a gallery or in a media player.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param curr_msg_id  This is the current message
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
uint32_t        dc_get_next_media            (dc_context_t*, uint32_t msg_id, int dir, int msg_type, int or_msg_type2, int or_msg_type3);


/**
 * Archive or unarchive a chat.
 *
 * Archived chats are not included in the default chatlist returned
 * by dc_get_chatlist().  Instead, if there are _any_ archived chats,
 * the pseudo-chat with the chat_id DC_CHAT_ID_ARCHIVED_LINK will be added the the
 * end of the chatlist.
 *
 * - To get a list of archived chats, use dc_get_chatlist() with the flag DC_GCL_ARCHIVED_ONLY.
 * - To find out the archived state of a given chat, use dc_chat_get_archived()
 * - Messages in archived chats are marked as being noticed, so they do not count as "fresh"
 * - Calling this function usually results in the event #DC_EVENT_MSGS_CHANGED
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to archive or unarchive.
 * @param archive 1=archive chat, 0=unarchive chat, all other values are reserved for future use
 * @return None.
 */
void            dc_archive_chat              (dc_context_t*, uint32_t chat_id, int archive);


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
void            dc_delete_chat               (dc_context_t*, uint32_t chat_id);


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
dc_array_t*     dc_get_chat_contacts         (dc_context_t*, uint32_t chat_id);


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
dc_array_t*     dc_search_msgs               (dc_context_t*, uint32_t chat_id, const char* query);


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
dc_chat_t*      dc_get_chat                  (dc_context_t*, uint32_t chat_id);


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
 * @param chat_name The name of the group chat to create.
 *     The name may be changed later using dc_set_chat_name().
 *     To find out the name of a group later, see dc_chat_get_name()
 * @return The chat ID of the new group chat, 0 on errors.
 */
uint32_t        dc_create_group_chat         (dc_context_t*, int verified, const char* name);


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
int             dc_is_contact_in_chat        (dc_context_t*, uint32_t chat_id, uint32_t contact_id);


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
int             dc_add_contact_to_chat       (dc_context_t*, uint32_t chat_id, uint32_t contact_id);


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
int             dc_remove_contact_from_chat  (dc_context_t*, uint32_t chat_id, uint32_t contact_id);


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
 * @param new_name New name of the group.
 * @param context The context as created by dc_context_new().
 * @return 1=success, 0=error
 */
int             dc_set_chat_name             (dc_context_t*, uint32_t chat_id, const char* name);


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
 * @param new_image Full path of the image to use as the group image.  If you pass NULL here,
 *     the group image is deleted (for promoted groups, all members are informed about this change anyway).
 * @return 1=success, 0=error
 */
int             dc_set_chat_profile_image    (dc_context_t*, uint32_t chat_id, const char* image);


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
 * @return Text string, must be free()'d after usage
 */
char*           dc_get_msg_info              (dc_context_t*, uint32_t msg_id);


/**
 * Get the raw mime-headers of the given message.
 * Raw headers are saved for incoming messages
 * only if `dc_set_config(context, "save_mime_headers", "1")`
 * was called before.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param msg_id The message id, must be the id of an incoming message.
 * @return Raw headers as a multi-line string, must be free()'d after usage.
 *     Returns NULL if there are no headers saved for the given message,
 *     eg. because of save_mime_headers is not set
 *     or the message is not incoming.
 */
char*           dc_get_mime_headers          (dc_context_t*, uint32_t msg_id);


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
void            dc_delete_msgs               (dc_context_t*, const uint32_t* msg_ids, int msg_cnt);


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
void            dc_forward_msgs              (dc_context_t*, const uint32_t* msg_ids, int msg_cnt, uint32_t chat_id);


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
void            dc_marknoticed_contact       (dc_context_t*, uint32_t contact_id);


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
void            dc_markseen_msgs             (dc_context_t*, const uint32_t* msg_ids, int msg_cnt);


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
void            dc_star_msgs                 (dc_context_t*, const uint32_t* msg_ids, int msg_cnt, int star);


/**
 * Get the total number of messages in a chat.
 *
 * @memberof dc_context_t
 * @param context The context object as returned from dc_context_new().
 * @param chat_id The ID of the chat to count the messages for.
 * @return Number of total messages in the given chat. 0 for errors or empty chats.
 */
dc_msg_t*       dc_get_msg                   (dc_context_t*, uint32_t msg_id);


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
 * Known and unblocked contacts will be returned by dc_get_contacts().
 *
 * To validate an e-mail address independently of the contact database
 * use dc_may_be_valid_addr().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param addr The e-mail-address to check.
 * @return 1=address is a contact in use, 0=address is not a contact in use.
 */
uint32_t        dc_lookup_contact_id_by_addr (dc_context_t*, const char* addr);


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
uint32_t        dc_create_contact            (dc_context_t*, const char* name, const char* addr);


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
 * the event DC_EVENT_CONTACTS_CHANGED is sent.
 *
 * To add a single contact entered by the user, you should prefer dc_create_contact(),
 * however, for adding a bunch of addresses, this function is _much_ faster.
 *
 * @memberof dc_context_t
 * @param context the context object as created by dc_context_new().
 * @param adr_book A multi-line string in the format
 *     `Name one\nAddress one\nName two\nAddress two`.
 *      If an email address already exists, the name is updated
 *      unless it was edited manually by dc_create_contact() before.
 * @return The number of modified or added contacts.
 */
int             dc_add_address_book          (dc_context_t*, const char*);


/**
 * Returns known and unblocked contacts.
 *
 * To get information about a single contact, see dc_get_contact().
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param listflags A combination of flags:
 *     - if the flag DC_GCL_ADD_SELF is set, SELF is added to the list unless filtered by other parameters
 *     - if the flag DC_GCL_VERIFIED_ONLY is set, only verified contacts are returned.
 *       if DC_GCL_VERIFIED_ONLY is not set, verified and unverified contacts are returned.
 * @param query A string to filter the list.  Typically used to implement an
 *     incremental search.  NULL for no filtering.
 * @return An array containing all contact IDs.  Must be dc_array_unref()'d
 *     after usage.
 */
dc_array_t*     dc_get_contacts              (dc_context_t*, uint32_t flags, const char* query);


/**
 * Get the number of blocked contacts.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return The number of blocked contacts.
 */
int             dc_get_blocked_cnt           (dc_context_t*);


/**
 * Get blocked contacts.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @return An array containing all blocked contact IDs.  Must be dc_array_unref()'d
 *     after usage.
 */
dc_array_t*     dc_get_blocked_contacts      (dc_context_t*);


/**
 * Block or unblock a contact.
 * May result in a #DC_EVENT_CONTACTS_CHANGED event.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param contact_id The ID of the contact to block or unblock.
 * @param new_blocking 1=block contact, 0=unblock contact
 * @return None.
 */
void            dc_block_contact             (dc_context_t*, uint32_t contact_id, int block);


/**
 * Get encryption info for a contact.
 * Get a multi-line encryption info, containing your fingerprint and the
 * fingerprint of the contact, used eg. to compare the fingerprints for a simple out-of-band verification.
 *
 * @memberof dc_context_t
 * @param context The context object as created by dc_context_new().
 * @param contact_id ID of the contact to get the encryption info for.
 * @return Multi-line text, must be free()'d after usage.
 */
char*           dc_get_contact_encrinfo      (dc_context_t*, uint32_t contact_id);


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
int             dc_delete_contact            (dc_context_t*, uint32_t contact_id);


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
dc_contact_t*   dc_get_contact               (dc_context_t*, uint32_t contact_id);


// import/export and tools

#define         DC_IMEX_EXPORT_SELF_KEYS      1 // param1 is a directory where the keys are written to
#define         DC_IMEX_IMPORT_SELF_KEYS      2 // param1 is a directory where the keys are searched in and read from
#define         DC_IMEX_EXPORT_BACKUP        11 // param1 is a directory where the backup is written to
#define         DC_IMEX_IMPORT_BACKUP        12 // param1 is the file with the backup to import


/**
 * Import/export things.
 * For this purpose, the function creates a job that is executed in the IMAP-thread then;
 * this requires to call dc_perform_imap_jobs() regularly.
 *
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
void            dc_imex                      (dc_context_t*, int what, const char* param1, const char* param2);


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
 *     free(file);
 * }
 * ~~~
 *
 * @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param dir_name Directory to search backups in.
 * @return String with the backup file, typically given to dc_imex(), returned strings must be free()'d.
 *     The function returns NULL if no backup was found.
 */
char*           dc_imex_has_backup           (dc_context_t*, const char* dir);


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
 * @return The setup code. Must be free()'d after usage.
 *     On errors, eg. if the message could not be sent, NULL is returned.
 */
char*           dc_initiate_key_transfer     (dc_context_t*);


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
int             dc_continue_key_transfer     (dc_context_t*, uint32_t msg_id, const char* setup_code);


/**
 * Signal an ongoing process to stop.
 *
 * After that, dc_stop_ongoing_process() returns _without_ waiting
 * for the ongoing process to return.
 *
 * The ongoing process will return ASAP then, however, it may
 * still take a moment.  If in doubt, the caller may also decide to kill the
 * thread after a few seconds; eg. the process may hang in a
 * function not under the control of the core (eg. #DC_EVENT_HTTP_GET). Another
 * reason for dc_stop_ongoing_process() not to wait is that otherwise it
 * would be GUI-blocking and should be started in another thread then; this
 * would make things even more complicated.
 *
 * Typical ongoing processes are started by dc_configure(),
 * dc_initiate_key_transfer() or dc_imex(). As there is always at most only
 * one onging process at the same time, there is no need to define _which_ process to exit.
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @return None.
 */
void            dc_stop_ongoing_process      (dc_context_t*);


// out-of-band verification

#define         DC_QR_ASK_VERIFYCONTACT      200 // id=contact
#define         DC_QR_ASK_VERIFYGROUP        202 // text1=groupname
#define         DC_QR_FPR_OK                 210 // id=contact
#define         DC_QR_FPR_MISMATCH           220 // id=contact
#define         DC_QR_FPR_WITHOUT_ADDR       230 // test1=formatted fingerprint
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
 * - DC_QR_ADDR with dc_lot_t::id=Contact ID
 * - DC_QR_TEXT with dc_lot_t::text1=Text
 * - DC_QR_URL with dc_lot_t::text1=URL
 * - DC_QR_ERROR with dc_lot_t::text1=Error string
 *
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param qr The text of the scanned QR code.
 * @return Parsed QR code as an dc_lot_t object. The returned object must be
 *     freed using dc_lot_unref() after usage.
 */
dc_lot_t*       dc_check_qr                  (dc_context_t*, const char* qr);


/**
 * Get QR code text that will offer an secure-join verification.
 * The QR code is compatible to the OPENPGP4FPR format
 * so that a basic fingerprint comparison also works eg. with OpenKeychain.
 *
 * The scanning device will pass the scanned content to dc_check_qr() then;
 * if this function returns DC_QR_ASK_VERIFYCONTACT or DC_QR_ASK_VERIFYGROUP
 * an out-of-band-verification can be joined using dc_join_securejoin()
 *
 * @memberof dc_context_t
 * @param context The context object.
 * @param group_chat_id If set to a group-chat-id,
 *     the group-join-protocol is offered in the QR code;
 *     works for verified groups as well as for normal groups.
 *     If set to 0, the setup-Verified-contact-protocol is offered in the QR code.
 * @return Text that should go to the QR code,
 *     On errors, an empty QR code is returned, NULL is never returned.
 *     The returned string must be free()'d after usage.
 */
char*           dc_get_securejoin_qr         (dc_context_t*, uint32_t chat_id);


/**
 * Join an out-of-band-verification initiated on another device with dc_get_securejoin_qr().
 * This function is typically called when dc_check_qr() returns
 * lot.state=DC_QR_ASK_VERIFYCONTACT or lot.state=DC_QR_ASK_VERIFYGROUP.
 *
 * This function takes some time and sends and receives several messages.
 * You should call it in a separate thread; if you want to abort it, you should
 * call dc_stop_ongoing_process().
 *
 * @memberof dc_context_t
 * @param context The context object
 * @param qr The text of the scanned QR code. Typically, the same string as given
 *     to dc_check_qr().
 * @return Chat-id of the joined chat, the UI may redirect to the this chat.
 *     If the out-of-band verification failed or was aborted, 0 is returned.
 */
uint32_t        dc_join_securejoin           (dc_context_t*, const char* qr);


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
void        dc_send_locations_to_chat       (dc_context_t*, uint32_t chat_id, int seconds);


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
int         dc_is_sending_locations_to_chat (dc_context_t*, uint32_t chat_id);


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
int         dc_set_location                 (dc_context_t*, double latitude, double longitude, double accuracy);


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
 * @param timestamp_from Start of timespan to return.
 *     Must be given in number of seconds since 00:00 hours, Jan 1, 1970 UTC.
 *     0 for "start from the beginning".
 * @param timestamp_to End of timespan to return.
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
dc_array_t* dc_get_locations                (dc_context_t*, uint32_t chat_id, uint32_t contact_id, int64_t timestamp_begin, int64_t timestamp_end);


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
void        dc_delete_all_locations         (dc_context_t*);


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
void             dc_array_unref              (dc_array_t*);


/**
 * Find out the number of items in an array.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @return Returns the number of items in a dc_array_t object. 0 on errors or if the array is empty.
 */
size_t           dc_array_get_cnt            (const dc_array_t*);


/**
 * Get the item at the given index as an ID.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item to get. Must be between 0 and dc_array_get_cnt()-1.
 * @return Returns the item at the given index. Returns 0 on errors or if the array is empty.
 */
uint32_t         dc_array_get_id             (const dc_array_t*, size_t index);


/**
 * Return the latitude of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Latitude of the item at the given index.
 *     0.0 if there is no latitude bound to the given item,
 */
double           dc_array_get_latitude       (const dc_array_t*, size_t index);


/**
 * Return the longitude of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Latitude of the item at the given index.
 *     0.0 if there is no longitude bound to the given item,
 */
double           dc_array_get_longitude      (const dc_array_t*, size_t index);


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
double           dc_array_get_accuracy       (const dc_array_t*, size_t index);


/**
 * Return the timestamp of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Timestamp of the item at the given index.
 *     0 if there is no timestamp bound to the given item,
 */
int64_t           dc_array_get_timestamp      (const dc_array_t*, size_t index);


/**
 * Return the chat-id of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Chat-id of the item at the given index.
 *     0 if there is no chat-id bound to the given item,
 */
uint32_t         dc_array_get_chat_id        (const dc_array_t*, size_t index);


/**
 * Return the contact-id of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Contact-id of the item at the given index.
 *     0 if there is no contact-id bound to the given item,
 */
uint32_t         dc_array_get_contact_id     (const dc_array_t*, size_t index);


/**
 * Return the message-id of the item at the given index.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return Message-id of the item at the given index.
 *     0 if there is no message-id bound to the given item,
 */
uint32_t         dc_array_get_msg_id         (const dc_array_t*, size_t index);


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
 *     The returned value must be free()'d after usage.
 */
char*            dc_array_get_marker         (const dc_array_t*, size_t index);


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
int              dc_array_is_independent     (const dc_array_t*, size_t index);


/**
 * Check if a given ID is present in an array.
 *
 * @private @memberof dc_array_t
 * @param array The array object to search in.
 * @param needle The ID to search for.
 * @param[out] ret_index If set, this will receive the index. Set to NULL if you're not interested in the index.
 * @return 1=ID is present in array, 0=ID not found.
 */
int              dc_array_search_id          (const dc_array_t*, uint32_t needle, size_t* indx);


/**
 * Get raw pointer to the data.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @return Raw pointer to the array. You MUST NOT free the data. You MUST NOT access the data beyond the current item count.
 *     It is not possible to enlarge the array this way.  Calling any other dc_array*()-function may discard the returned pointer.
 */
const uint32_t*  dc_array_get_raw            (const dc_array_t*);


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
void             dc_chatlist_unref           (dc_chatlist_t*);


/**
 * Find out the number of chats in a chatlist.
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist object as created eg. by dc_get_chatlist().
 * @return Returns the number of items in a dc_chatlist_t object. 0 on errors or if the list is empty.
 */
size_t           dc_chatlist_get_cnt         (const dc_chatlist_t*);


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
uint32_t         dc_chatlist_get_chat_id     (const dc_chatlist_t*, size_t index);


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
uint32_t         dc_chatlist_get_msg_id      (const dc_chatlist_t*, size_t index);


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
dc_lot_t*        dc_chatlist_get_summary     (const dc_chatlist_t*, size_t index, dc_chat_t*);


/**
 * Helper function to get the associated context object.
 *
 * @memberof dc_chatlist_t
 * @param chatlist The chatlist object to empty.
 * @return Context object associated with the chatlist. NULL if none or on errors.
 */
dc_context_t*    dc_chatlist_get_context     (dc_chatlist_t*);


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


void            dc_chat_unref                (dc_chat_t*);

uint32_t        dc_chat_get_id               (const dc_chat_t*);
int             dc_chat_get_type             (const dc_chat_t*);
char*           dc_chat_get_name             (const dc_chat_t*);
char*           dc_chat_get_subtitle         (const dc_chat_t*);
char*           dc_chat_get_profile_image    (const dc_chat_t*);
uint32_t        dc_chat_get_color            (const dc_chat_t*);
int             dc_chat_get_archived         (const dc_chat_t*);
int             dc_chat_is_unpromoted        (const dc_chat_t*);
int             dc_chat_is_self_talk         (const dc_chat_t*);
int             dc_chat_is_verified          (const dc_chat_t*);
int             dc_chat_is_sending_locations (const dc_chat_t*);


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


dc_msg_t*       dc_msg_new                    (dc_context_t*, int viewtype);
void            dc_msg_unref                  (dc_msg_t*);
uint32_t        dc_msg_get_id                 (const dc_msg_t*);
uint32_t        dc_msg_get_from_id            (const dc_msg_t*);
uint32_t        dc_msg_get_chat_id            (const dc_msg_t*);
int             dc_msg_get_viewtype           (const dc_msg_t*);
int             dc_msg_get_state              (const dc_msg_t*);
int64_t          dc_msg_get_timestamp          (const dc_msg_t*);
int64_t          dc_msg_get_received_timestamp (const dc_msg_t*);
int64_t          dc_msg_get_sort_timestamp     (const dc_msg_t*);
char*           dc_msg_get_text               (const dc_msg_t*);
char*           dc_msg_get_file               (const dc_msg_t*);
char*           dc_msg_get_filename           (const dc_msg_t*);
char*           dc_msg_get_filemime           (const dc_msg_t*);
uint64_t        dc_msg_get_filebytes          (const dc_msg_t*);
int             dc_msg_get_width              (const dc_msg_t*);
int             dc_msg_get_height             (const dc_msg_t*);
int             dc_msg_get_duration           (const dc_msg_t*);
int             dc_msg_get_showpadlock        (const dc_msg_t*);
dc_lot_t*       dc_msg_get_summary            (const dc_msg_t*, const dc_chat_t*);
char*           dc_msg_get_summarytext        (const dc_msg_t*, int approx_characters);
int             dc_msg_has_deviating_timestamp(const dc_msg_t*);
int             dc_msg_has_location           (const dc_msg_t*);
int             dc_msg_is_sent                (const dc_msg_t*);
int             dc_msg_is_starred             (const dc_msg_t*);
int             dc_msg_is_forwarded           (const dc_msg_t*);
int             dc_msg_is_info                (const dc_msg_t*);
int             dc_msg_is_increation          (const dc_msg_t*);
int             dc_msg_is_setupmessage        (const dc_msg_t*);
char*           dc_msg_get_setupcodebegin     (const dc_msg_t*);
void            dc_msg_set_text               (dc_msg_t*, const char* text);
void            dc_msg_set_file               (dc_msg_t*, const char* file, const char* filemime);
void            dc_msg_set_dimension          (dc_msg_t*, int width, int height);
void            dc_msg_set_duration           (dc_msg_t*, int duration);
void            dc_msg_set_location           (dc_msg_t*, double latitude, double longitude);
void            dc_msg_latefiling_mediasize   (dc_msg_t*, int width, int height, int duration);


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
#define         DC_CONTACT_ID_DEVICE         2
#define         DC_CONTACT_ID_LAST_SPECIAL   9


void            dc_contact_unref             (dc_contact_t*);
uint32_t        dc_contact_get_id            (const dc_contact_t*);
char*           dc_contact_get_addr          (const dc_contact_t*);
char*           dc_contact_get_name          (const dc_contact_t*);
char*           dc_contact_get_display_name  (const dc_contact_t*);
char*           dc_contact_get_name_n_addr   (const dc_contact_t*);
char*           dc_contact_get_first_name    (const dc_contact_t*);
char*           dc_contact_get_profile_image (const dc_contact_t*);
uint32_t        dc_contact_get_color         (const dc_contact_t*);
int             dc_contact_is_blocked        (const dc_contact_t*);
int             dc_contact_is_verified       (dc_contact_t*);


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


void            dc_lot_unref             (dc_lot_t*);
char*           dc_lot_get_text1         (const dc_lot_t*);
char*           dc_lot_get_text2         (const dc_lot_t*);
int             dc_lot_get_text1_meaning (const dc_lot_t*);
int             dc_lot_get_state         (const dc_lot_t*);
uint32_t        dc_lot_get_id            (const dc_lot_t*);
int64_t          dc_lot_get_timestamp     (const dc_lot_t*);


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
 * @defgroup DC_EVENT DC_EVENT
 *
 * These constants are used as events
 * reported to the callback given to dc_context_new().
 * If you do not want to handle an event, it is always safe to return 0,
 * so there is no need to add a "case" for every event.
 *
 * @addtogroup DC_EVENT
 * @{
 */


/**
 * The library-user may write an informational string to the log.
 * Passed to the callback given to dc_context_new().
 *
 * This event should not be reported to the end-user using a popup or something like that.
 *
 * @param data1 0
 * @param data2 (const char*) Info string in english language.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @return 0
 */
#define DC_EVENT_INFO                     100


/**
 * Emitted when SMTP connection is established and login was successful.
 *
 * @param data1 0
 * @param data2 (const char*) Info string in english language.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @return 0
 */
#define DC_EVENT_SMTP_CONNECTED           101


/**
 * Emitted when IMAP connection is established and login was successful.
 *
 * @param data1 0
 * @param data2 (const char*) Info string in english language.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @return 0
 */
#define DC_EVENT_IMAP_CONNECTED           102

/**
 * Emitted when a message was successfully sent to the SMTP server.
 *
 * @param data1 0
 * @param data2 (const char*) Info string in english language.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @return 0
 */
#define DC_EVENT_SMTP_MESSAGE_SENT        103


/**
 * The library-user should write a warning string to the log.
 * Passed to the callback given to dc_context_new().
 *
 * This event should not be reported to the end-user using a popup or something like that.
 *
 * @param data1 0
 * @param data2 (const char*) Warning string in english language.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @return 0
 */
#define DC_EVENT_WARNING                  300


/**
 * The library-user should report an error to the end-user.
 * Passed to the callback given to dc_context_new().
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
 * @param data2 (const char*) Error string, always set, never NULL. Frequent error strings are
 *     localized using #DC_EVENT_GET_STRING, however, most error strings will be in english language.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @return 0
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
 * For this purpose, data1 is set to 1 if the error is probably worth reporting.
 *
 * Moreover, if the UI detects that the device is offline,
 * it is probably more useful to report this to the user
 * instead of the string from data2.
 *
 * @param data1 (int) 1=first/new network error, should be reported the user;
 *     0=subsequent network error, should be logged only
 * @param data2 (const char*) Error string, always set, never NULL.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @return 0
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
 * @param data2 (const char*) Info string in english language.
 *     Must not be free()'d or modified
 *     and is valid only until the callback returns.
 * @return 0
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
 * @return 0
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
 * @return 0
 */
#define DC_EVENT_INCOMING_MSG             2005


/**
 * A single message is sent successfully. State changed from  DC_STATE_OUT_PENDING to
 * DC_STATE_OUT_DELIVERED, see dc_msg_get_state().
 *
 * @param data1 (int) chat_id
 * @param data2 (int) msg_id
 * @return 0
 */
#define DC_EVENT_MSG_DELIVERED            2010


/**
 * A single message could not be sent. State changed from DC_STATE_OUT_PENDING or DC_STATE_OUT_DELIVERED to
 * DC_STATE_OUT_FAILED, see dc_msg_get_state().
 *
 * @param data1 (int) chat_id
 * @param data2 (int) msg_id
 * @return 0
 */
#define DC_EVENT_MSG_FAILED               2012


/**
 * A single message is read by the receiver. State changed from DC_STATE_OUT_DELIVERED to
 * DC_STATE_OUT_MDN_RCVD, see dc_msg_get_state().
 *
 * @param data1 (int) chat_id
 * @param data2 (int) msg_id
 * @return 0
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
 * @return 0
 */
#define DC_EVENT_CHAT_MODIFIED            2020


/**
 * Contact(s) created, renamed, blocked or deleted.
 *
 * @param data1 (int) If not 0, this is the contact_id of an added contact that should be selected.
 * @param data2 0
 * @return 0
 */
#define DC_EVENT_CONTACTS_CHANGED         2030



/**
 * Location of one or more contact has changed.
 *
 * @param data1 (int) contact_id of the contact for which the location has changed.
 *     If the locations of several contacts have been changed,
 *     eg. after calling dc_delete_all_locations(), this parameter is set to 0.
 * @param data2 0
 * @return 0
 */
#define DC_EVENT_LOCATION_CHANGED         2035


/**
 * Inform about the configuration progress started by dc_configure().
 *
 * @param data1 (int) 0=error, 1-999=progress in permille, 1000=success and done
 * @param data2 0
 * @return 0
 */
#define DC_EVENT_CONFIGURE_PROGRESS       2041


/**
 * Inform about the import/export progress started by dc_imex().
 *
 * @param data1 (int) 0=error, 1-999=progress in permille, 1000=success and done
 * @param data2 0
 * @return 0
 */
#define DC_EVENT_IMEX_PROGRESS            2051


/**
 * A file has been exported. A file has been written by dc_imex().
 * This event may be sent multiple times by a single call to dc_imex().
 *
 * A typical purpose for a handler of this event may be to make the file public to some system
 * services.
 *
 * @param data1 (const char*) Path and file name.
 *     Must not be free()'d or modified and is valid only until the callback returns.
 * @param data2 0
 * @return 0
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
 * @return 0
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
 * @return 0
 */
#define DC_EVENT_SECUREJOIN_JOINER_PROGRESS       2061


// the following events are functions that should be provided by the frontends


/**
 * Requeste a localized string from the frontend.
 *
 * @param data1 (int) ID of the string to request, one of the DC_STR_* constants.
 * @param data2 (int) The count. If the requested string contains a placeholder for a numeric value,
 *     the ui may use this value to return different strings on different plural forms.
 * @return (const char*) Null-terminated UTF-8 string.
 *     The string will be free()'d by the core,
 *     so it must be allocated using malloc() or a compatible function.
 *     Return 0 if the ui cannot provide the requested string
 *     the core will use a default string in english language then.
 */
#define DC_EVENT_GET_STRING               2091

/**
 * @}
 */

#define DC_EVENT_FILE_COPIED         2055 // deprecated
#define DC_EVENT_IS_OFFLINE          2081 // deprecated
#define DC_ERROR_SEE_STRING          0    // deprecated
#define DC_ERROR_SELF_NOT_IN_GROUP   1    // deprecated
#define DC_STR_SELFNOTINGRP          21   // deprecated
#define DC_EVENT_DATA1_IS_STRING(e)  ((e)==DC_EVENT_IMEX_FILE_WRITTEN || (e)==DC_EVENT_FILE_COPIED)
#define DC_EVENT_DATA2_IS_STRING(e)  ((e)>=100 && (e)<=499)
#define DC_EVENT_RETURNS_INT(e)      ((e)==DC_EVENT_IS_OFFLINE)
#define DC_EVENT_RETURNS_STRING(e)   ((e)==DC_EVENT_GET_STRING)
char*           dc_get_version_str           (void); // deprecated
void            dc_array_add_id              (dc_array_t*, uint32_t); // deprecated


/*
 * Values for dc_get|set_config("show_emails")
 */
#define DC_SHOW_EMAILS_OFF               0
#define DC_SHOW_EMAILS_ACCEPTED_CONTACTS 1
#define DC_SHOW_EMAILS_ALL               2


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
#define DC_STR_MEMBER                     4
#define DC_STR_CONTACT                    6
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
#define DC_STR_SELFTALK_SUBTITLE          50
#define DC_STR_CANNOT_LOGIN               60
#define DC_STR_SERVER_RESPONSE            61
#define DC_STR_MSGACTIONBYUSER            62
#define DC_STR_MSGACTIONBYME              63
#define DC_STR_MSGLOCATIONENABLED         64
#define DC_STR_MSGLOCATIONDISABLED        65
#define DC_STR_LOCATION                   66
#define DC_STR_COUNT                      66

void dc_str_unref (char*);


/*
 * @}
 */


#ifdef __cplusplus
}
#endif
#endif // __DELTACHAT_H__
