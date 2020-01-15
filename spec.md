# Chat-over-Email specification

Version 0.20.0

This document describes how emails can be used
to implement typical messenger functions
while staying compatible to existing MUAs.

- [Encryption](#encryption)
- [Outgoing messages](#outgoing-messages)
- [Incoming messages](#incoming-messages)
- [Forwarded messages](#forwarded-messages)
- [Groups](#groups)
    - [Outgoing group messages](#outgoing-group-messages)
    - [Incoming group messages](#incoming-group-messages)
    - [Add and remove members](#add-and-remove-members)
    - [Change group name](#change-group-name)
    - [Set group image](#set-group-image)
- [Set profile image](#set-profile-image)
- [Miscellaneous](#miscellaneous)


# Encryption

Messages SHOULD be encrypted by the
[Autocrypt](https://autocrypt.org/level1.html) standard;
`prefer-encrypt=mutual` MAY be set by default.

Meta data (at least the subject and all chat-headers) SHOULD be encrypted
by the [Memoryhole](https://github.com/autocrypt/memoryhole) standard.
If Memoryhole is not used,
the subject of encrypted messages SHOULD be replaced by the string
`Chat: Encrypted message` where the part after the colon MAY be localized.


# Outgoing messages

Messengers MUST add a `Chat-Version: 1.0` header to outgoing messages.
For filtering and smart appearance of the messages in normal MUAs,
the `Subject` header SHOULD start with the characters `Chat:`
and SHOULD be an excerpt of the message.
Replies to messages MAY follow the typical `Re:`-format.

The body MAY contain text which MUST have the content type `text/plain`
or `mulipart/alternative` containing `text/plain`.

The text MAY be divided into a user-text-part and a footer-part using the
line `-- ` (minus, minus, space, lineend).

The user-text-part MUST contain only user generated content.
User generated content are eg. texts a user has actually typed
or pasted or forwarded from another user.
Full quotes, footers or sth. like that MUST NOT go to the user-text-part.

    From: sender@domain
    To: rcpt@domain
    Chat-Version: 1.0
    Content-Type: text/plain
    Subject: Chat: Hello ...

    Hello world!


# Incoming messages

The `Chat-Version` header MAY be used
to detect if a messages comes from a compatible messenger.

The `Subject` header MUST NOT be used
to detect compatible messengers, groups or whatever.

Messenger SHOULD show the `Subject`
if the message comes from a normal MUA together with the email-body.
The email-body SHOULD be converted
to plain text, full-quotes and similar regions SHOULD be cut.

Attachments SHOULD be shown where possible.
If an attachment cannot be shown, a non-distracting warning SHOULD be printed.


# Forwarded messages

Forwarded messages are outgoing messages that contain a forwarded-header
before the user generated content.

The forwarded header MUST contain two lines: 
The first line contains the text
`---------- Forwarded message ----------`
(10 minus, space, text `Forwarded message`, space, 10 minus). 
The second line starts with `From: ` followed by the original sender
which SHOULD be anonymized or just a placeholder.

    From: sender@domain
    To: rcpt@domain
    Chat-Version: 1.0
    Content-Type: text/plain
    Subject: Chat: Forwarded message

    ---------- Forwarded message ----------
    From: Messenger

    Hello world!

Incoming forwarded messages are detected by the header.
The messenger SHOULD mark these messages in a way that
it becomes obvious that the message is not created by the sender.
Note that most messengers do not show the original sender of forwarded messages
but MUAs typically expose the sender in the UI.


# Groups

Groups are chats with usually more than one recipient,
each defined by an email-address.
The sender plus the recipients are the group members.

To allow different groups with the same members,
groups are identified by a group-id.
The group-id MUST be created only from the characters
`0`-`9`, `A`-`Z`, `a`-`z` `_` and `-`
and MUST have a length of at least 11 characters.

Groups MUST have a group-name.
The group-name is any non-zero-length UTF-8 string.

Groups MAY have a group-image.


## Outgoing groups messages

All group members MUST be added to the `From`/`To` headers.
The group-id MUST be written to the `Chat-Group-ID` header.
The group-name MUST be written to `Chat-Group-Name` header
(the forced presence of this header makes it easier
to join a group chat on a second device any time).

The `Subject` header of outgoing group messages
SHOULD start with the characters `Chat:`
followed by the group-name and a colon followed by an excerpt of the message.

To identify the group-id on replies from normal MUAs,
the group-id MUST also be added to the message-id of outgoing messages.
The message-id MUST have the format `Gr.<group-id>.<unique data>`.

    From: member1@domain
    To: member2@domain, member3@domain
    Chat-Version: 1.0
    Chat-Group-ID: 12345uvwxyZ
    Chat-Group-Name: My Group
    Message-ID: Gr.12345uvwxyZ.0001@domain
    Subject: Chat: My Group: Hello group ...

    Hello group - this group contains three members

Messengers adding the member list in the form `Name <email-address>`
MUST take care only to distribute the names authorized by the contacts themselves.
Otherwise, names as _Daddy_ or _Honey_ may be distributed
(this issue is also true for normal MUAs, however,
for more contact- and chat-centralized apps
such situations happen more frequently).


## Incoming group messages

The messenger MUST search incoming messages for the group-id
in the following headers: `Chat-Group-ID`,
`Message-ID`, `In-Reply-To` and `References` (in this order).

If the messenger finds a valid and existent group-id,
the message SHOULD be assigned to the given group.
If the messenger finds a valid but not existent group-id,
the messenger MAY create a new group.
If no group-id is found,
the message MAY be assigned
to a normal single-user chat with the email-address given in `From`.


## Add and remove members

Messenger clients MUST construct the member list
from the `From`/`To` headers only on the first group message
or if they see a `Chat-Group-Member-Added`
or `Chat-Group-Member-Removed` action header.
Both headers MUST have the email-address
of the added or removed member as the value.
Messenger clients MUST NOT construct the member list
on other group messages
(this is to avoid accidentally altered To-lists in normal MUAs;
the user does not expect adding a user to a _message_
will also add him to the _group_ "forever").

The messenger SHOULD send an explicit mail for each added or removed member.
The body of the message SHOULD contain
a localized description about what happened
and the message SHOULD appear as a message or action from the sender.

    From: member1@domain
    To: member2@domain, member3@domain, member4@domain
    Chat-Version: 1.0
    Chat-Group-ID: 12345uvwxyZ
    Chat-Group-Name: My Group
    Chat-Group-Member-Added: member4@domain
    Message-ID: Gr.12345uvwxyZ.0002@domain
    Subject: Chat: My Group: Hello, ...

    Hello, I've added member4@domain to our group.  Now we have 4 members.

To remove a member:

    From: member1@domain
    To: member2@domain, member3@domain
    Chat-Version: 1.0
    Chat-Group-ID: 12345uvwxyZ
    Chat-Group-Name: My Group
    Chat-Group-Member-Removed: member4@domain
    Message-ID: Gr.12345uvwxyZ.0003@domain
    Subject: Chat: My Group: Hello, ...

    Hello, I've removed member4@domain from our group.  Now we have 3 members.


## Change group name

To change the group-name,
the messenger MUST send the action header `Chat-Group-Name-Changed`
with the value set to the old group name to all group members.
The new group name goes to the header `Chat-Group-Name`.

The messenger SHOULD send an explicit mail for each name change.
The body of the message SHOULD contain
a localized description about what happened
and the message SHOULD appear as a message or action from the sender.

    From: member1@domain
    To: member2@domain, member3@domain
    Chat-Version: 1.0
    Chat-Group-ID: 12345uvwxyZ
    Chat-Group-Name: Our Group
    Chat-Group-Name-Changed: My Group
    Message-ID: Gr.12345uvwxyZ.0004@domain
    Subject: Chat: Our Group: Hello, ...

    Hello, I've changed the group name from "My Group" to "Our Group".


## Set group image

A group MAY have a group-image.
To change or set the group-image,
the messenger MUST attach an image file to a message
and MUST add the header `Chat-Group-Avatar`
with the value set to the image name.

To remove the group-image,
the messenger MUST add the header `Chat-Group-Avatar: 0`.

The messenger SHOULD send an explicit mail for each group image change.
The body of the message SHOULD contain
a localized description about what happened
and the message SHOULD appear as a message or action from the sender.


    From: member1@domain
    To: member2@domain, member3@domain
    Chat-Version: 1.0
    Chat-Group-ID: 12345uvwxyZ
    Chat-Group-Name: Our Group
    Chat-Group-Avatar: image.jpg
    Message-ID: Gr.12345uvwxyZ.0005@domain
    Subject: Chat: Our Group: Hello, ...
    Content-Type: multipart/mixed; boundary="==break=="

    --==break==
    Content-Type: text/plain

    Hello, I've changed the group image.
    --==break==
    Content-Type: image/jpeg
    Content-Disposition: attachment; filename="image.jpg"

    /9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAYEBQYFBAYGBQYHBw ...
    --==break==--

The image format SHOULD be image/jpeg or image/png.
To save data, it is RECOMMENDED
to add a `Chat-Group-Avatar` only on image changes.


# Set profile image

A user MAY have a profile-image that MAY be distributed to their contacts.
To change or set the profile-image,
the messenger MUST attach an image file to a message
and MUST add the header `Chat-User-Avatar`
with the value set to the image name.

To remove the profile-image,
the messenger MUST add the header `Chat-User-Avatar: 0`.

To distribute the image,
the messenger MAY send the profile image
together with the next mail to a given contact
(to do this only once,
the messenger has to keep a `user_avatar_update_state` somewhere).
Alternatively, the messenger MAY send an explicit mail
for each profile-image change to all contacts using a compatible messenger.
The messenger SHOULD NOT send an explicit mail to normal MUAs.

    From: sender@domain
    To: rcpt@domain
    Chat-Version: 1.0
    Chat-User-Avatar: photo.jpg
    Subject: Chat: Hello, ...
    Content-Type: multipart/mixed; boundary="==break=="

    --==break==
    Content-Type: text/plain

    Hello, I've changed my profile image.
    --==break==
    Content-Type: image/jpeg
    Content-Disposition: attachment; filename="photo.jpg"

    AKCgkJi3j4l5kjoldfUAKCgkJi3j4lldfHjgWICwgIEBQYFBA ...
    --==break==--

The image format SHOULD be image/jpeg or image/png.
Note that `Chat-User-Avatar` may appear together with all other headers,
eg. there may be a `Chat-User-Avatar` and a `Chat-Group-Avatar` header
in the same message.
To save data, it is RECOMMENDED to add a `Chat-User-Avatar` header
only on image changes.


# Miscellaneous

Messengers SHOULD use the header `In-Reply-To` as usual.

Messengers SHOULD add a `Chat-Voice-message: 1` header
if an attached audio file is a voice message.

Messengers MAY add a `Chat-Duration` header
to specify the duration of attached audio or video files.
The value MUST be the duration in milliseconds.
This allows the receiver to show the time without knowing the file format.

    In-Reply-To: Gr.12345uvwxyZ.0005@domain
    Chat-Voice-Message: 1
    Chat-Duration: 10000

Messengers MAY send and receive Message Disposition Notifications
(MDNs, [RFC 8098](https://tools.ietf.org/html/rfc8098),
[RFC 3503](https://tools.ietf.org/html/rfc3503))
using the `Chat-Disposition-Notification-To` header
instead of the `Disposition-Notification-To`
(which unfortunately forces many other MUAs
to send weird mails not following any standard).


## Sync messages

If some action is required by a message header,
the action should only be performed if the _effective date_ is newer
than the date the last action was performed.

We define the effective date of a message
as the sending time of the message as indicated by its Date header,
or the time of first receipt if that date is in the future or unavailable.


Copyright © 2017-2019 Delta Chat contributors.
