# Changelog 

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

