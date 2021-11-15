# Changelog

## 1.65.0

### Changes
- python: add mypy support and some type hints #2809

### Fixes
- do not disable ephemeral timer when downloading a message partially #2811
- apply existing ephemeral timer also to partially downloaded messages;
  after full download, the ephemeral timer starts over #2811
- replace user-visible error on verification failure with warning;
  the error is logged to the corresponding chat anyway #2808


## 1.64.0

### Fixes
- add 'waiting for being added to the group' only for group-joins,
  not for setup-contact #2797
- prioritize In-Reply-To: and References: headers over group IDs when assigning
  messages to chats to fix incorrect assignment of Delta Chat replies to
  classic email threads #2795


## 1.63.0

### API changes
- `dc_get_last_error()` added #2788

### Changes
- Optimize Autocrypt gossip #2743

### Fixes
- fix permanently hiding of one-to-one chats after secure-join #2791


## 1.62.0

### API Changes
- `dc_join_securejoin()` now always returns immediately;
  the returned chat may not allow sending (`dc_chat_can_send()` returns false)
  which may change as usual on `DC_EVENT_CHAT_MODIFIED` #2508 #2767
- introduce multi-device-sync-messages;
  as older cores display them as files in self-chat,
  they are currently only sent if config option `send_sync_msgs` is set #2669
- add `DC_EVENT_SELFAVATAR_CHANGED` #2742

### Changes
- use system DNS instead of google for MX queries #2780
- improve error logging #2758
- improve tests #2764 #2781
- improve ci #2770
- refactorings #2677 #2728 #2740 #2729 #2766 #2778

### Fixes
- add Let's Encrypt certificate to core as it may be missing older devices #2752
- prioritize certificate setting from user over the one from provider-db #2749
- fix "QR process failed" error #2725
- do not update quota in endless loop #2726


## 1.61.0

### API Changes
- download-on-demand added: `dc_msg_get_download_state()`, `dc_download_full_msg()`
  and `download_limit` config option #2631 #2696
- `dc_create_broadcast_list()` and chat type `DC_CHAT_TYPE_BROADCAST` added #2707 #2722
- allow ui-specific configs using `ui.`-prefix in key (`dc_set_config(context, "ui.*", value)`) #2672
- new strings from `DC_STR_PARTIAL_DOWNLOAD_MSG_BODY`
  to `DC_STR_PART_OF_TOTAL_USED` #2631 #2694 #2707 #2723
- emit warnings and errors from account manager with account-id 0 #2712

### Changes
- notify about incoming contact requests #2690
- messages are marked as read on first read receipt #2699
- quota warning reappears after import, rewarning at 95% #2702
- lock strict TLS if certificate checks are automatic #2711
- always check certificates strictly when connecting over SOCKS5 in Automatic mode #2657
- `Accounts` is not cloneable anymore #2654 #2658
- update chat/contact data only when there was no newer update #2642
- better detection of mailing list names #2665 #2685
- log all decisions when applying ephemeral timer to chats #2679
- connectivity view now translatable #2694 #2723
- improve Doxygen documentation #2647 #2668 #2684 #2688 #2705
- refactorings #2656 #2659 #2677 #2673 #2678 #2675 #2663 #2692 #2706
- update provider database #2618

### Fixes
- ephemeral timer rollback protection #2693 #2709
- recreate configured folders if they are deleted #2691
- ignore MDNs sent to self #2674
- recognize NDNs that put headers into "message/global-headers" part #2598
- avoid `dc_get_contacts()` returning duplicate contact ids #2591
- do not leak group names on forwarding messages #2719
- in case of smtp-errors, iterate over all addresses to fix ipv6/v4 problems #2720
- fix pkg-config file #2660
- fix "QR process failed" error #2725


## 1.60.0

### Added
- add device message to warn about QUOTA #2621
- add SOCKS5 support #2474 #2620

### Changes
- don't emit multiple events with the same import/export progress number #2639
- reduce message length limit to 5000 chars #2615

### Fixes
- keep event emitter from closing when there are no accounts #2636


## 1.59.0

### Added
- add quota information to `dc_get_connectivity_html()`

### Changes
- refactorings #2592 #2570 #2581
- add 'device chat about' to now existing status #2613
- update provider database #2608

### Fixes
- provider database supports socket=PLAIN and dotless domains now #2604 #2608
- add migrated accounts to events emitter #2607
- fix forwarding quote-only mails #2600
- do not set WantsMdn param for outgoing messages #2603
- set timestamps for system messages #2593
- do not treat gmail labels as folders #2587
- avoid timing problems in `dc_maybe_network_lost()` #2551
- only set smtp to "connected" if the last message was actually sent #2541


## 1.58.0

### Fixes
- move WAL file together with database
  and avoid using data if the database was not closed correctly before #2583


## 1.57.0

### API Changes

- breaking change: removed deaddrop chat #2514 #2563

  Contact request chats are not merged into a single virtual
  "deaddrop" chat anymore. Instead, they are shown in the chatlist the
  same way as other chats, but sending of messages to them is not
  allowed and MDNs are not sent automatically until the chat is
  "accepted" by the user.

  New API:
  - `dc_chat_is_contact_request()`: returns true if chat is a contact
    request.  In this case an option to accept the chat via
    `dc_accept_chat()` should be shown in the UI.
  - `dc_accept_chat()`: unblock the chat or accept contact request
  - `dc_block_chat()`: block the chat, currently works only for mailing
    lists.

  Removed API:
  - `dc_create_chat_by_msg_id()`: deprecated 2021-02-07 in favor of
    `dc_decide_on_contact_request()`
  - `dc_marknoticed_contact()`: deprecated 2021-02-07 in favor of
    `dc_decide_on_contact_request()`
  - `dc_decide_on_contact_request()`: this call requires a message ID
    from deaddrop chat as input. As deaddrop chat is removed, this
    call can't be used anymore.
  - `dc_msg_get_real_chat_id()`: use `dc_msg_get_chat_id()` instead, the
    only difference between these calls was in handling of deaddrop
    chat
  - removed `DC_CHAT_ID_DEADDROP` and `DC_STR_DEADDROP` constants

- breaking change: removed `DC_EVENT_ERROR_NETWORK` and `DC_STR_SERVER_RESPONSE`
  Instead, there is a new api `dc_get_connectivity()`
  and `dc_get_connectivity_html()`;
  `DC_EVENT_CONNECTIVITY_CHANGED` is emitted on changes

- breaking change: removed `dc_accounts_import_account()`
  Instead you need to add an account and call `dc_imex(DC_IMEX_IMPORT_BACKUP)`
  on its context

- update account api, 2 new methods:
  `int dc_all_work_done (dc_context_t* context);`
  `int dc_accounts_all_work_done (dc_accounts_t* accounts);`

- add api to check if a message was `Auto-Submitted`
  cffi: `int dc_msg_is_bot (const dc_msg_t* msg);`
  python: `Message.is_bot()`

- `dc_context_t* dc_accounts_get_selected_account (dc_accounts_t* accounts);`
  now returns `NULL` if there is no selected account

- added `dc_accounts_maybe_network_lost()` for systems core cannot find out
  connectivity loss on its own (eg. iOS) #2550

### Added
- use Auto-Submitted: auto-generated header to identify bots #2502
- allow sending stickers via repl tool
- chat: make `get_msg_cnt()` and `get_fresh_msg_cnt()` work for deaddrop chat #2493
- withdraw/revive own qr-codes #2512
- add Connectivity view (a better api for getting the connection status) #2319 #2549 #2542

### Changes
- updated spec: new `Chat-User-Avatar` usage, `Chat-Content: sticker`, structure, copyright year #2480
- update documentation #2548 #2561 #2569
- breaking: `Accounts::create` does not also create an default account anymore #2500
- remove "forwarded" from stickers, as the primary way of getting stickers
  is by asking a bot and then forwarding them currently #2526
- mimeparser: use mailparse to parse RFC 2231 filenames #2543
- allow email addresses without dot in the domain part #2112
- allow installing lib and include under different prefixes #2558
- remove counter from name provided by `DC_CHAT_ID_ARCHIVED_LINK` #2566
- improve tests #2487 #2491 #2497
- refactorings #2492 #2503 #2504 #2506 #2515 #2520 #2567 #2575 #2577 #2579
- improve ci #2494
- update provider-database #2565

### Removed
- remove `dc_accounts_import_account()` api #2521
- remove `DC_EVENT_ERROR_NETWORK` and `DC_STR_SERVER_RESPONSE` #2319

### Fixes
- allow stickers with gif-images #2481
- fix database migration #2486
- do not count hidden messages in get_msg_cnt(). #2493
- improve drafts detection #2489
- fix panic when removing last, selected account from account manager #2500
- set_draft's message-changed-event returns now draft's msg id instead of 0 #2304
- avoid hiding outgoing classic emails #2505
- fixes for message timestamps #2517
- do not process names, avatars, location XMLs, message signature etc.
  for duplicate messages #2513
- fix `can_send` for users not in group #2479
- fix receiving events for accounts added by `dc_accounts_add_account()` #2559
- fix which chats messages are assigned to #2465
- fix: don't create chats when MDNs are received #2578


## 1.56.0

- fix downscaling images #2469

- fix outgoing messages popping up in selfchat #2456

- securejoin: display error reason if there is any #2470

- do not allow deleting contacts with ongoing chats #2458

- fix: ignore drafts folder when scanning #2454

- fix: scan folders also when inbox is not watched #2446

- more robust In-Reply-To parsing #2182

- update dependencies #2441 #2438 #2439 #2440 #2447 #2448 #2449 #2452 #2453 #2460 #2464 #2466

- update provider-database #2471

- refactorings #2459 #2457

- improve tests and ci #2445 #2450 #2451


## 1.55.0

- fix panic when receiving some HTML messages #2434

- fix downloading some messages multiple times #2430

- fix formatting of read receipt texts #2431

- simplify SQL error handling #2415

- explicit rust API for creating chats with blocked status #2282

- debloat the binary by using less AsRef arguments #2425


## 1.54.0

- switch back from `sqlx` to `rusqlite` due to performance regressions #2380 #2381 #2385 #2387

- global search performance improvement #2364 #2365 #2366

- improve SQLite performance with `PRAGMA synchronous=normal` #2382

- python: fix building of bindings against system-wide install of `libdeltachat` #2383 #2385

- python: list `requests` as a requirement #2390

- fix creation of many delete jobs when being offline #2372

- synchronize status between devices #2386

- deaddrop (contact requests) chat improvements #2373

- add "Forwarded:" to notification and chatlist summaries #2310

- place user avatar directly into `Chat-User-Avatar` header #2232 #2384

- improve tests #2360 #2362 #2370 #2377 #2387

- cleanup #2359 #2361 #2374 #2376 #2379 #2388


## 1.53.0

- fix sqlx performance regression #2355 2356

- add a `ci_scripts/coverage.sh` #2333 #2334

- refactorings and tests #2348 #2349 #2350

- improve python bindings #2332 #2326


## 1.52.0

- database library changed from rusqlite to sqlx #2089 #2331 #2336 #2340

- add alias support: UIs should check for `dc_msg_get_override_sender_name()`
  also in single-chats now and display divergent names and avatars #2297

- parse blockquote-tags for better quote detection #2313

- ignore unknown classical emails from spam folder #2311

- support "Mixed Up” encryption repairing #2321

- fix single chat search #2344

- fix nightly clippy and rustc errors #2341

- update dependencies #2350

- improve ci #2342

- improve python bindings #2332 #2326


## 1.51.0

- breaking change: You have to call `dc_stop_io()`/`dc_start_io()`
  before/after `dc_imex(DC_IMEX_EXPORT_BACKUP)`:
  fix race condition and db corruption
  when a message was received during backup #2253

- save subject for messages: new api `dc_msg_get_subject()`,
  when quoting, use the subject of the quoted message as the new subject,
  instead of the last subject in the chat #2274 #2283

- new apis to get full or html message,
  `dc_msg_has_html()` and `dc_get_msg_html()` #2125 #2151 #2264 #2279

- new chat type and apis for the new mailing list support,
  `DC_CHAT_TYPE_MAILINGLIST`, `dc_msg_get_real_chat_id()`,
  `dc_msg_get_override_sender_name()` #1964 #2181 #2185 #2195 #2211 #2210 #2240
  #2241 #2243 #2258 #2259 #2261 #2267 #2270 #2272 #2290

- new api `dc_decide_on_contact_request()`,
  deprecated `dc_create_chat_by_msg_id()` and `dc_marknoticed_contact()` #1964

- new flag `DC_GCM_INFO_ONLY` for api `dc_get_chat_msgs()` #2132

- new api `dc_get_chat_encrinfo()` #2186

- new api `dc_contact_get_status()`, returning the recent footer #2218 #2307

- improve contact name update rules,
  add api `dc_contact_get_auth_name()` #2206 #2212 #2225

- new api for bots: `dc_msg_set_html()` #2153

- new api for bots: `dc_msg_set_override_sender_name()` #2231

- api removed: `dc_is_io_running()` #2139

- api removed: `dc_contact_get_first_name()` #2165 #2171

- improve compatibility with providers changing the Message-ID
  (as Outlook.com) #2250 #2265

- correctly show emails that were sent to an alias and then bounced 

- implement Consistent Color Generation (XEP-0392),
  that results in contact colors be be changed #2228 #2229 #2239

- fetch recent existing messages
  and create corresponding chats after configure #2106

- improve e-mail compatibility
  by scanning all folders from time to time #2067 #2152 #2158 #2184 #2215 #2224

- better support videochat-services not supporting random rooms #2191

- export backups as .tar files #2023

- scale avatars based on media_quality, fix avatar rotation #2063

- compare ephemeral timer to parent message to deal with reordering better #2100

- better ephemeral system messages #2183

- read quotes out of html messages #2104

- prepend subject to messages with attachments, if needed #2111

- run housekeeping at least once a day #2114

- resolve MX domain only once per OAuth2 provider #2122

- configure provider based on MX record #2123 #2134

- make transient bad destination address error permanent
  after n tries #2126 #2202

- enable strict TLS for known providers by default #2121

- improve and harden secure join #2154 #2161 #2251

- update `dc_get_info()` to return more information #2156

- prefer In-Reply-To/References
  over group-id stored in Message-ID #2164 #2172 #2173

- apply gossiped encryption preference to new peerstates #2174

- fix: do not return quoted messages from the trash chat #2221

- fix: allow emojis for location markers #2177

- fix encoding of Chat-Group-Name-Changed messages that could even lead to
  messages not being delivered #2141

- fix error when no temporary directory is available #1929

- fix marking read receipts as seen #2117

- fix read-notification for mixed-case addresses #2103

- fix decoding of attachment filenames #2080 #2094 #2102

- fix downloading ranges of message #2061

- fix parsing quoted encoded words in From: header #2193 #2204

- fix import/export race condition #2250

- fix: exclude muted chats from notified-list #2269 #2275

- fix: update uid_next if the server rewind it #2288

- fix: return error on fingerprint mismatch on qr-scan #2295

- fix ci #2217 #2226 #2244 #2245 #2249 #2277 #2286

- try harder on backup opening #2148

- trash messages more thoroughly #2273

- nicer logging #2284

- add CMakeLists.txt #2260

- switch to rust 1.50, update toolchains, deps #2150 #2155 #2165 #2107 #2262 #2271

- improve python bindings #2113 #2115 #2133 #2214

- improve documentation #2143 #2160 #2175 #2146

- refactorings #2110 #2136 #2135 #2168 #2178 #2189 #2190 #2198 #2197 #2201 #2196
  #2200 #2230 #2262 #2203

- update provider-database #2299


## 1.50.0

- do not fetch emails in between inbox_watch disabled and enabled again #2087

- fix: do not fetch from INBOX if inbox_watch is disabled #2085

- fix: do not use STARTTLS when PLAIN connection is requested
  and do not allow downgrade if STARTTLS is not available #2071


## 1.49.0

- add timestamps to image and video filenames #2068

- forbid quoting messages from another context #2069

- fix: preserve quotes in messages with attachments #2070


## 1.48.0

- `fetch_existing` renamed to `fetch_existing_msgs` and disabled by default
  #2035 #2042

- skip fetch existing messages/contacts if config-option `bot` set #2017

- always log why a message is sorted to trash #2045

- display a quote if top posting is detected #2047

- add ephemeral task cancellation to `dc_stop_io()`;
  before, there was no way to quickly terminate pending ephemeral tasks #2051

- when saved-messages chat is deleted,
  a device-message about recreation is added #2050

- use `max_smtp_rcpt_to` from provider-db,
  sending messages to many recipients in configurable chunks #2056

- fix handling of empty autoconfigure files #2027

- fix adding saved messages to wrong chats on multi-device #2034 #2039

- fix hang on android4.4 and other systems
  by adding a workaround to executer-blocking-handling bug #2040

- fix secret key export/import roundtrip #2048

- fix mistakenly unarchived chats #2057

- fix outdated-reminder test that fails only 7 days a year,
  including halloween :) #2059

- improve python bindings #2021 #2036 #2038

- update provider-database #2037


## 1.47.0

- breaking change: `dc_update_device_chats()` removed;
  this is now done automatically during configure
  unless the new config-option `bot` is set #1957

- breaking change: split `DC_EVENT_MSGS_NOTICED` off `DC_EVENT_MSGS_CHANGED`
  and remove `dc_marknoticed_all_chats()` #1942 #1981

- breaking change: remove unused starring options #1965

- breaking change: `DC_CHAT_TYPE_VERIFIED_GROUP` replaced by
  `dc_chat_is_protected()`; also single-chats may be protected now, this may
  happen over the wire even if the UI do not offer an option for that #1968

- breaking change: split quotes off message text,
  UIs should use at least `dc_msg_get_quoted_text()` to show quotes now #1975

- new api for quote handling: `dc_msg_set_quote()`, `dc_msg_get_quoted_text()`,
  `dc_msg_get_quoted_msg()` #1975 #1984 #1985 #1987 #1989 #2004

- require quorum to enable encryption #1946

- speed up and clean up account creation #1912 #1927 #1960 #1961

- configure now collects recent contacts and fetches last messages
  unless disabled by `fetch_existing` config-option #1913 #2003
  EDIT: `fetch_existing` renamed to `fetch_existing_msgs` in 1.48.0 #2042

- emit `DC_EVENT_CHAT_MODIFIED` on contact rename
  and set contact-id on `DC_EVENT_CONTACTS_CHANGED` #1935 #1936 #1937

- add `dc_set_chat_protection()`; the `protect` parameter in
  `dc_create_group_chat()` will be removed in an upcoming release;
  up to then, UIs using the "verified group" paradigm
  should not use `dc_set_chat_protection()` #1968 #2014 #2001 #2012 #2007

- remove unneeded `DC_STR_COUNT` #1991

- mark all failed messages as failed when receiving an NDN #1993

- check some easy cases for bad system clock and outdated app #1901

- fix import temporary directory usage #1929

- fix forcing encryption for reset peers #1998

- fix: do not allow to save drafts in non-writeable chats #1997

- fix: do not show HTML if there is no content and there is an attachment #1988

- fix recovering offline/lost connections, fixes background receive bug #1983

- fix ordering of accounts returned by `dc_accounts_get_all()` #1909

- fix whitespace for summaries #1938

- fix: improve sentbox name guessing #1941

- fix: avoid manual poll impl for accounts events #1944

- fix encoding newlines in param as a preparation for storing quotes #1945

- fix: internal and ffi error handling #1967 #1966 #1959 #1911 #1916 #1917 #1915

- fix ci #1928 #1931 #1932 #1933 #1934 #1943

- update provider-database #1940 #2005 #2006

- update dependencies #1919 #1908 #1950 #1963 #1996 #2010 #2013


## 1.46.0

- breaking change: `dc_configure()` report errors in
  `DC_EVENT_CONFIGURE_PROGRESS`: capturing error events is no longer working
  #1886 #1905

- breaking change: removed `DC_LP_{IMAP|SMTP}_SOCKET*` from `server_flags`;
  added `mail_security` and `send_security` using `DC_SOCKET` enum #1835

- parse multiple servers in Mozilla autoconfig #1860

- try multiple servers for each protocol #1871

- do IMAP and SMTP configuration in parallel #1891

- configuration cleanup and speedup #1858 #1875 #1889 #1904 #1906

- secure-join cleanup, testing, fixing #1876 #1877 #1887 #1888 #1896 #1899 #1900

- do not reset peerstate on encrypted messages,
  ignore reordered autocrypt headers #1885 #1890

- always sort message replies after parent message #1852

- add an index to significantly speed up `get_fresh_msg_cnt()` #1881

- improve mimetype guessing for PDF and many other formats #1857 #1861

- improve accepting invalid html #1851

- improve tests, cleanup and ci #1850 #1856 #1859 #1861 #1884 #1894 #1895

- tweak HELO command #1908

- make `dc_accounts_get_all()` return accounts sorted #1909

- fix KML coordinates precision used for location streaming #1872

- fix cancelling import/export #1855


## 1.45.0

- add `dc_accounts_t` account manager object and related api functions #1784

- add capability to import backups as .tar files,
  which will become the default in a subsequent release #1749

- try various server domains on configuration #1780 #1838

- recognize .tgs files as stickers #1826

- remove X-Mailer debug header #1819

- improve guessing message types from extension #1818

- fix showing unprotected subjects in encrypted messages #1822

- fix threading in interaction with non-delta-clients #1843

- fix handling if encryption degrades #1829

- fix webrtc-servers names set by the user #1831

- update provider database #1828

- update async-imap to fix Oauth2 #1837

- optimize jpeg assets with trimage #1840

- add tests and documentations #1809 #1820


## 1.44.0

- fix peerstate issues #1800 #1805

- fix a crash related to muted chats #1803

- fix incorrect dimensions sometimes reported for images #1806

- fixed `dc_chat_get_remaining_mute_duration` function #1807

- handle empty tags (e.g. `<br/>`) in HTML mails #1810

- always translate the message about disappearing messages timer change #1813

- improve footer detection in plain text email #1812

- update device chat icon to fix warnings in iOS logs #1802

- fix deletion of multiple messages #1795


## 1.43.0

- improve using own jitsi-servers #1785

- fix smtp-timeout tweaks for larger mails #1797

- more bug fixes and updates #1794 #1792 #1789 #1787


## 1.42.0

- new qr-code type `DC_QR_WEBRTC` #1779

- new `dc_chatlist_get_summary2()` api #1771

- tweak smtp-timeout for larger mails #1782

- optimize read-receipts #1765

- Allow http scheme for DCACCOUNT URLs #1770

- improve tests #1769

- bug fixes #1766 #1772 #1773 #1775 #1776 #1777


## 1.41.0

- new apis to initiate video chats #1718 #1735

- new apis `dc_msg_get_ephemeral_timer()`
  and `dc_msg_get_ephemeral_timestamp()`

- new api `dc_chatlist_get_summary2()` #1771

- improve IMAP handling #1703 #1704

- improve ephemeral messages #1696 #1705

- mark location-messages as auto-generated #1715

- multi-device avatar-sync #1716 #1717

- improve python bindings #1732 #1733 #1738 #1769

- Allow http scheme for DCACCOUNT urls #1770

- more fixes #1702 #1706 #1707 #1710 #1719 #1721
  #1723 #1734 #1740 #1744 #1748 #1760 #1766 #1773 #1765

- refactorings #1712 #1714 #1757

- update toolchains and dependencies #1726 #1736 #1737 #1742 #1743 #1746


## 1.40.0

- introduce ephemeral messages #1540 #1680 #1683 #1684 #1691 #1692

- `DC_MSG_ID_DAYMARKER` gets timestamp attached #1677 #1685

- improve idle #1690 #1688

- fix message processing issues by sequential processing #1694

- refactorings #1670 #1673


## 1.39.0

- fix handling of `mvbox_watch`, `sentbox_watch`, `inbox_watch` #1654 #1658

- fix potential panics, update dependencies #1650 #1655


## 1.38.0

- fix sorting, esp. for multi-device


## 1.37.0

- improve ndn heuristics #1630

- get oauth2 authorizer from provider-db #1641

- removed linebreaks and spaces from generated qr-code #1631

- more fixes #1633 #1635 #1636 #1637


## 1.36.0

- parse ndn (network delivery notification) reports
  and report failed messages as such #1552 #1622 #1630

- add oauth2 support for gsuite domains #1626

- read image orientation from exif before recoding #1619

- improve logging #1593 #1598

- improve python and bot bindings #1583 #1609

- improve imap logout #1595

- fix sorting #1600 #1604

- fix qr code generation #1631

- update rustcrypto releases #1603

- refactorings #1617


## 1.35.0

- enable strict-tls from a new provider-db setting #1587

- new subject 'Message from USER' for one-to-one chats #1395

- recode images #1563

- improve reconnect handling #1549 #1580

- improve importing addresses #1544

- improve configure and folder detection #1539 #1548

- improve test suite #1559 #1564 #1580 #1581 #1582 #1584 #1588:

- fix ad-hoc groups #1566

- preventions against being marked as spam #1575

- refactorings #1542 #1569


## 1.34.0

- new api for io, thread and event handling #1356,
  see the example atop of `deltachat.h` to get an overview

- LOTS of speed improvements due to async processing #1356

- enable WAL mode for sqlite #1492

- process incoming messages in bulk #1527

- improve finding out the sent-folder #1488

- several bug fixes


## 1.33.0

- let `dc_set_muted()` also mute one-to-one chats #1470

- fix a bug that led to load and traffic if the server does not use sent-folder
  #1472


## 1.32.0

- fix endless loop when trying to download messages with bad RFC Message-ID,
  also be more reliable on similar errors #1463 #1466 #1462

- fix bug with comma in contact request #1438

- do not refer to hidden messages on replies #1459

- improve error handling #1468 #1465 #1464


## 1.31.0

- always describe the context of the displayed error #1451

- do not emit `DC_EVENT_ERROR` when message sending fails;
  `dc_msg_get_state()` and `dc_get_msg_info()` are sufficient #1451

- new config-option `media_quality` #1449

- try over if writing message to database fails #1447


## 1.30.0

- expunge deleted messages #1440

- do not send `DC_EVENT_MSGS_CHANGED|INCOMING_MSG` on hidden messages #1439


## 1.29.0

- new config options `delete_device_after` and `delete_server_after`,
  each taking an amount of seconds after which messages
  are deleted from the device and/or the server #1310 #1335 #1411 #1417 #1423

- new api `dc_estimate_deletion_cnt()` to estimate the effect
  of `delete_device_after` and `delete_server_after`

- use Ed25519 keys by default, these keys are much shorter
  than RSA keys, which results in saving traffic and speed improvements #1362

- improve message ellipsizing #1397 #1430

- emit `DC_EVENT_ERROR_NETWORK` also on smtp-errors #1378

- do not show badly formatted non-delta-messages as empty #1384

- try over SMTP on potentially recoverable error 5.5.0 #1379

- remove device-chat from forward-to-chat-list #1367

- improve group-handling #1368

- `dc_get_info()` returns uptime (how long the context is in use)

- python improvements and adaptions #1408 #1415

- log to the stdout and stderr in tests #1416

- refactoring, code improvements #1363 #1365 #1366 #1370 #1375 #1389 #1390 #1418 #1419

- removed api: `dc_chat_get_subtitle()`, `dc_get_version_str()`, `dc_array_add_id()`

- removed events: `DC_EVENT_MEMBER_ADDED`, `DC_EVENT_MEMBER_REMOVED`


## 1.28.0

- new flag DC_GCL_FOR_FORWARDING for dc_get_chatlist()
  that will sort the "saved messages" chat to the top of the chatlist #1336
- mark mails as being deleted from server in dc_empty_server() #1333
- fix interaction with servers that do not allow folder creation on root-level;
  use path separator as defined by the email server #1359
- fix group creation if group was created by non-delta clients #1357
- fix showing replies from non-delta clients #1353
- fix member list on rejoining left groups #1343
- fix crash when using empty groups #1354
- fix potential crash on special names #1350


## 1.27.0

- handle keys reliably on armv7 #1327


## 1.26.0

- change generated key type back to RSA as shipped versions
  have problems to encrypt to Ed25519 keys

- update rPGP to encrypt reliably to Ed25519 keys;
  one of the next versions can finally use Ed25519 keys then


## 1.25.0

- save traffic by downloading only messages that are really displayed #1236

- change generated key type to Ed25519, these keys are much shorter
  than RSA keys, which results in saving traffic and speed improvements #1287

- improve key handling #1237 #1240 #1242 #1247

- mute handling, apis are dc_set_chat_mute_duration()
  dc_chat_is_muted() and dc_chat_get_remaining_mute_duration() #1143

- pinning chats, new apis are dc_set_chat_visibility() and
  dc_chat_get_visibility() #1248

- add dc_provider_new_from_email() api that queries the new, integrated
  provider-database #1207

- account creation by scanning a qr code
  in the DCACCOUNT scheme (https://mailadm.readthedocs.io),
  new api is dc_set_config_from_qr() #1249

- if possible, dc_join_securejoin(), returns the new chat-id immediately
  and does the handshake in background #1225

- update imap and smtp dependencies #1115

- check for MOVE capability before using MOVE command #1263

- allow inline attachments from RFC 2183 #1280

- fix updating names from incoming mails #1298

- fix error messages shown on import #1234

- directly attempt to re-connect if the smtp connection is maybe stale #1296

- improve adding group members #1291

- improve rust-api #1261

- cleanup #1302 #1283 #1282 #1276 #1270-#1274 #1267 #1258-#1260
  #1257 #1239 #1231 #1224

- update spec #1286 #1291


## 1.0.0-beta.24

- fix oauth2/gmail bug introduced in beta23 (not used in releases) #1219

- fix panic when receiving eg. cyrillic filenames #1216

- delete all consumed secure-join handshake messagess #1209 #1212

- rust-level cleanups #1218 #1217 #1210 #1205

- python-level cleanups #1204 #1202 #1201


## 1.0.0-beta.23

- #1197 fix imap-deletion of messages 

- #1171 Combine multiple MDNs into a single mail, reducing traffic 

- #1155 fix to not send out gossip always, reducing traffic

- #1160 fix reply-to-encrypted determination 

- #1182 Add "Auto-Submitted: auto-replied" header to MDNs

- #1194 produce python wheels again, fix c/py.delta.chat
  master-deployment 

- rust-level housekeeping and improvements #1161 #1186 #1185 #1190 #1194 #1199 #1191 #1190 #1184 and more

- #1063 clarify licensing 

- #1147 use mailparse 0.10.2 


## 1.0.0-beta.22

- #1095 normalize email lineends to CRLF

- #1095 enable link-time-optimization, saves eg. on android 11 mb

- #1099 fix import regarding devicechats

- #1092 improve logging

- #1096 #1097 #1094 #1090 #1091 internal cleanups

## 1.0.0-beta.21

- #1078 #1082 ensure RFC compliance by producing 78 column lines for
  encoded attachments. 

- #1080 don't recreate and thus break group membership if an unknown 
  sender (or mailer-daemon) sends a message referencing the group chat 

- #1081 #1079 some internal cleanups 

- update imap-proto dependency, to fix yandex/oauth 

## 1.0.0-beta.20

- #1074 fix OAUTH2/gmail
- #1072 fix group members not appearing in contact list
- #1071 never block interrupt_idle (thus hopefully also not on maybe_network())
- #1069 reduce smtp-timeout to 30 seconds
- #1066 #1065 avoid unwrap in dehtml, make literals more readable

## 1.0.0-beta.19

- #1058 timeout smtp-send if it doesn't complete in 15 minutes 

- #1059 trim down logging

## 1.0.0-beta.18

- #1056 avoid panicking when we couldn't read imap-server's greeting
  message 

- #1055 avoid panicking when we don't have a selected folder

- #1052 #1049 #1051 improve logging to add thread-id/name and
  file/lineno to each info/warn message.

- #1050 allow python bindings to initialize Account with "os_name".


## 1.0.0-beta.17

- #1044 implement avatar recoding to 192x192 in core to keep file sizes small. 

- #1024 fix #1021 SQL/injection malformed Chat-Group-Name breakage

- #1036 fix smtp crash by pulling in a fixed async-smtp 

- #1039 fix read-receipts appearing as normal messages when you change
  MDN settings 

- #1040 do not panic on SystemTimeDifference

- #1043 avoid potential crashes in malformed From/Chat-Disposition... headers  

- #1045 #1041 #1038 #1035 #1034 #1029 #1025 various cleanups and doc
  improvments 

## 1.0.0-beta.16

- alleviate login problems with providers which only
  support RSA1024 keys by switching back from Rustls 
  to native-tls, by using the new async-email/async-native-tls 
  crate from @dignifiedquire. thanks @link2xt. 

- introduce per-contact profile images to send out 
  own profile image heuristically, and fix sending
  out of profile images in "in-prepare" groups. 
  this also extends the Chat-spec that is maintained
  in core to specify Chat-Group-Image and Chat-Group-Avatar
  headers. thanks @r10s and @hpk42.

- fix merging of protected headers from the encrypted
  to the unencrypted parts, now not happening recursively
  anymore.  thanks @hpk and @r10s

- fix/optimize autocrypt gossip headers to only get 
  sent when there are more than 2 people in a chat. 
  thanks @link2xt

- fix displayname to use the authenticated name 
  when available (displayname as coming from contacts 
  themselves). thanks @simon-laux

- introduce preliminary support for offline autoconfig 
  for nauta provider. thanks @hpk42 @r10s

## 1.0.0-beta.15

- fix #994 attachment appeared doubled in chats (and where actually
  downloaded after smtp-send). @hpk42

## 1.0.0-beta.14

- fix packaging issue with our rust-email fork, now we are tracking
  master again there. hpk42

## 1.0.0-beta.13

- fix #976 -- unicode-issues in display-name of email addresses. @hpk42

- fix #985 group add/remove member bugs resulting in broken groups.  @hpk42

- fix hanging IMAP connections -- we now detect with a 15second timeout
  if we cannot terminate the IDLE IMAP protocol. @hpk42 @link2xt

- fix incoming multipart/mixed containing html, to show up as
  attachments again.  Fixes usage for simplebot which sends html
  files for users to interact with the bot. @adbenitez @hpk42 

- refinements to internal autocrypt-handling code, do not send
  prefer-encrypt=nopreference as it is the default if no attribute
  is present.  @linkxt 

- simplify, modularize and rustify several parts 
  of dc-core (general WIP). @link2xt @flub @hpk42 @r10s

- use async-email/async-smtp to handle SMTP connections, might
  fix connection/reconnection issues. @link2xt 

- more tests and refinements for dealing with blobstorage @flub @hpk42 

- use a dedicated build-server for CI testing of core PRs


## 1.0.0-beta.12

- fix python bindings to use core for copying attachments to blobdir
  and fix core to actually do it. @hpk42

## 1.0.0-beta.11

- trigger reconnect more often on imap error states.  Should fix an 
  issue observed when trying to empty a folder.  @hpk42

- un-split qr tests: we fixed qr-securejoin protocol flakyness 
  last weeks. @hpk42

## 1.0.0-beta.10

- fix grpid-determination from in-reply-to and references headers. @hpk42

- only send Autocrypt-gossip headers on encrypted messages. @dignifiedquire

- fix reply-to-encrypted message to also be encrypted. @hpk42

- remove last unsafe code from dc_receive_imf :) @hpk42

- add experimental new dc_chat_get_info_json FFI/API so that desktop devs
  can play with using it. @jikstra

- fix encoding of subjects and attachment-filenames @hpk42
  @dignifiedquire . 

## 1.0.0-beta.9

- historic: we now use the mailparse crate and lettre-email to generate mime
  messages.  This got rid of mmime completely, the C2rust generated port of the libetpan 
  mime-parse -- IOW 22KLocs of cumbersome code removed! see 
  https://github.com/deltachat/deltachat-core-rust/pull/904#issuecomment-561163330
  many thanks @dignifiedquire for making everybody's life easier 
  and @jonhoo (from rust-imap fame) for suggesting to use the mailparse crate :) 

- lots of improvements and better error handling in many rust modules 
  thanks @link2xt @flub @r10s, @hpk42 and @dignifiedquire 

- @r10s introduced a new device chat which has an initial
  welcome message.  See 
  https://c.delta.chat/classdc__context__t.html#a1a2aad98bd23c1d21ee42374e241f389
  for the main new FFI-API.

- fix moving self-sent messages, thanks @r10s, @flub, @hpk42

- fix flakyness/sometimes-failing verified/join-protocols, 
  thanks @flub, @r10s, @hpk42

- fix reply-to-encrypted message to keep encryption 

- new DC_EVENT_SECUREJOIN_MEMBER_ADDED event 

- many little fixes and rustifications (@link2xt, @flub, @hpk42)


## 1.0.0-beta.8

- now uses async-email/async-imap as the new base 
  which makes imap-idle interruptible and thus fixes
  several issues around the imap thread being in zombie state . 
  thanks @dignifiedquire, @hpk42 and @link2xt. 

- fixes imap-protocol parsing bugs that lead to infinitely
  repeated crashing while trying to receive messages with
  a subjec that contained non-utf8. thanks @link2xt

- fixed logic to find encryption subkey -- previously 
  delta chat would use the primary key for encryption
  (which works with RSA but not ECC). thanks @link2xt

- introduce a new device chat where core and UIs can 
  add "device" messages.  Android uses it for an initial
  welcome message. thanks @r10s

- fix time smearing (when two message are virtually send
  in the same second, there would be misbehaviour because
  we didn't persist smeared time). thanks @r10s

- fix double-dotted extensions like .html.zip or .tar.gz  
  to not mangle them when creating blobfiles.  thanks @flub

- fix backup/exports where the wrong sql file would be modified,
  leading to problems when exporting twice.  thanks @hpk42

- several other little fixes and improvements 


## 1.0.0-beta.7

- fix location-streaming #782

- fix display of messages that could not be decrypted #785
 
- fix smtp MAILER-DAEMON bug #786 

- fix a logging of durations #783

- add more error logging #779

- do not panic on some bad utf-8 mime #776

## 1.0.0-beta.6

- fix chatlist.get_msg_id to return id, instead of wrongly erroring

## 1.0.0-beta.5

- fix dc_get_msg() to return empty messages when asked for special ones 

## 1.0.0-beta.4

- fix more than one sending of autocrypt setup message

- fix recognition of mailto-address-qr-codes, add tests

- tune down error to warning when adding self to chat

## 1.0.0-beta.3

- add back `dc_empty_server()` #682

- if `show_emails` is set to `DC_SHOW_EMAILS_ALL`,
  email-based contact requests are added to the chatlist directly

- fix IMAP hangs #717 and cleanups

- several rPGP fixes

- code streamlining and rustifications


## 1.0.0-beta.2

- https://c.delta.chat docs are now regenerated again through our CI 

- several rPGP cleanups, security fixes and better multi-platform support 

- reconnect on io errors and broken pipes (imap)

- probe SMTP with real connection not just setup

- various imap/smtp related fixes

- use to_string_lossy in most places instead of relying on valid utf-8
  encodings
 
- rework, rustify and test autoconfig-reading and parsing 

- some rustifications/boolifications of c-ints 


## 1.0.0-beta.1 

- first beta of the Delta Chat Rust core library. many fixes of crashes
  and other issues compared to 1.0.0-alpha.5.

- Most code is now "rustified" and does not do manual memory allocation anymore. 

- The `DC_EVENT_GET_STRING` event is not used anymore, removing the last
  event where the core requested a return value from the event callback. 

  Please now use `dc_set_stock_translation()` API for core messages
  to be properly localized. 

- Deltachat FFI docs are automatically generated and available here: 
  https://c.delta.chat 

- New events ImapMessageMoved and ImapMessageDeleted

For a full list of changes, please see our closed Pull Requests: 

https://github.com/deltachat/deltachat-core-rust/pulls?q=is%3Apr+is%3Aclosed
