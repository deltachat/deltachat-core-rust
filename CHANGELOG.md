# Changelog 

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

- new DC_EVENT_SECUREJOIN_SUCCEEDED event 

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

