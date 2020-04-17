# Mailing list integration of Delta Chat

A collection of information to help with the integration of mailing lists into Delta Chat.

## Chat name

How should the chat be titled?

It could be taken from the email header `List-Id`:

### Mailman 

* Schema: `${list-description} <${list-local-part}.${list-domain}>`
* $list-description could be many words, should be truncated. Could also be empty, though.

### Schleuder
* Schema: `<${list-local-part}.${list-domain}>`
* No name available.
* Could be absent: <https://0xacab.org/schleuder/schleuder/-/blob/master/lib/schleuder/mail/message.rb#L357-358>.


### Sympa
* Schema: `<${list-local-part}.${list-domain}>`
* No name available.
* Always present <https://github.com/sympa-community/sympa/blob/sympa-6.2/src/lib/Sympa/Spindle/TransformOutgoing.pm#L110-L118>, <https://github.com/sympa-community/sympa/blob/sympa-6.2/src/lib/Sympa/List.pm#L6832-L6833>.


## Sender name

What's the name of the person that actually sent the message?

Some MLM change the sender information to avoid problems with DMARC.

### Mailman

* Since v2.1.16 the `From` depends on the list's configuration and possibly on the sender-domain's DMARC-configuration — i.e. messages from the same list can arrive one of the Variants A-D!
* Documentation of config options:
 * <https://wiki.list.org/DOC/Mailman%202.1%20List%20Administrators%20Manual#line-544>,
 * <https://wiki.list.org/DOC/Mailman%202.1%20List%20Administrators%20Manual#line-163>,
 * <https://wiki.list.org/DEV/DMARC>
* Variant A: `From` is unchanged.
* Variant B. `From` is mangled like this: `${sender-name} via ${list-name} <${list-addr-spec}>`. The original sender is put into `Reply-To`.
* Variant C. `From` is mangled like this: `${sender-name} <${encoded-sender-addr-spec}@${list-domain}>`. The original sender is put into `Reply-To`.
* Variant D: `From` is set to ${list-name-addr}, the originally sent message is included as mime-part.
* Variant E: `From` is set to ${list-name-addr}, original sender information is removed.

### Schleuder

* Visible only in mime-body (which is possibly encrypted), or not at all.
* The first `text/plain` mime-part may include the information, as taken from the original message: `From: ${sender-name-addr}`.

### Sympa
* Depends on the list's configuration.
* Documentation:
 * <https://sympa-community.github.io/manual/customize/dmarc-protection.html>,
 * <https://sympa-community.github.io/gpldoc/man/sympa.conf.5.html#dmarc-protection>.
* Variant A: `From` is unchanged.
* Variant B: `From` is mangled like Mailman Variant B, but the original sender information is put into `X-Original-From`.


## Autocrypt

### Mailman
* Not supported.

### Schleuder
* A list's key is included in sent messages (as of version 3.5.0) (optional, by default active): <https://0xacab.org/schleuder/schleuder/-/blob/master/lib/schleuder/mail/message.rb#L349-355>.
* Incoming keys are not yet looked at (that feature is planned: <https://0xacab.org/schleuder/schleuder/issues/435>).

### Sympa
* Not supported.
