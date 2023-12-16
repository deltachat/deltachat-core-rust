AEAP MVP
========

Changes to the UIs
------------------

- The secondary self addresses (see below) are shown in the UI, but not editable.

- When the user changed the email address in the configure screen, show a dialog to the user, either directly explaining things or with a link to the FAQ (see "Other" below)

Changes in the core
-------------------

- [x] We have one primary self address and any number of secondary self addresses. `is_self_addr()` checks all of them.

- [x] If the user does a reconfigure and changes the email address, the previous address is added as a secondary self address.

  - don't forget to deduplicate secondary self addresses in case the user switches back and forth between addresses).

  - The key stays the same.

- [x] No changes for 1:1 chats, there simply is a new one. (This works since, contrary to group messages, messages sent to a 1:1 chat are not assigned to the group chat but always to the 1:1 chat with the sender. So it's not a problem that the new messages might be put into the old chat if they are a reply to a message there.)

- [ ] When sending a message: If any of the secondary self addrs is in the chat's member list, remove it locally (because we just transitioned away from it). We add a log message for this (alternatively, a system message in the chat would be more visible).

- [x] ([#3385](https://github.com/deltachat/deltachat-core-rust/pull/3385)) When receiving a message: If the key exists, but belongs to another address (we may want to benchmark this)
  AND there is a `Chat-Version` header\
  AND the message is signed correctly
  AND the From address is (also) in the encrypted (and therefore signed) headers <sup>[[1]](#myfootnote1)</sup>\
  AND the message timestamp is newer than the contact's `lastseen` (to prevent changing the address back when messages arrive out of order) (this condition is not that important since we will have eventual consistency even without it):

  Replace the contact in _all_ groups, possibly deduplicate the members list, and add a system message to all of these chats.
  
  - Note that we can't simply compare the keys byte-by-byte, since the UID may have changed, or the sender may have rotated the key and signed the new key with the old one.

<a name="myfootnote1">[1]</a>: Without this check, an attacker could replay a message from Alice to Bob. Then Bob's device would do an AEAP transition from Alice's to the attacker's address, allowing for easier phishing.

<details>
<summary>More details about this</summary>
Suppose Alice sends a message to Evil (or to a group with both Evil and Bob). Evil then forwards the message to Bob, changing the From and To headers (and if necessary Message-Id) and replacing `addr=alice@example.org;` in the autocrypt header with `addr=evil@example.org;`.

Then Bob's device sees that there is a message which is signed by Alice's key and comes from Evil's address and would do the AEAP transition, i.e. replace Alice with Evil in all groups and show a message "Alice changed their address from alice@example.org to evil@example.org". Disadvantages for Evil are that Bob's message will be shown on Alice's device, possibly creating confusion/suspicion, and that the usual "Setup changed for..." message will be shown the next time Evil sends a message (because Evil doesn't know Alice's private key).

Possible mitigations:
- if we make the AEAP device message sth. like "Automatically removed alice@example.org and added evil@example.org", then this will create more suspicion, making the phishing harder (we didn't talk about what what the wording should be at all yet).
- Add something similar to replay protection to our Autocrypt implementation. This could be done e.g. by adding a second `From` header to the protected headers. If it's present, the receiver then requires it to be the same as the outer `From`, and if it's not present, we don't do AEAP --> **That's what we implemented**

Note that usually a mail is signed by a key that has a UID matching the from address.

  That's not mandatory for Autocrypt (and in fact, we just keep the old UID when changing the self address, so with AEAP the UID will actually be different than the from address sometimes)

  https://autocrypt.org/level1.html#openpgp-based-key-data says:
  > The content of the user id packet is only decorative

</details>

### Notes:
  
- We treat protected and non-protected chats the same
- We leave the aeap transition statement away since it seems not to be needed, makes things harder on the sending side, wastes some network traffic, and is worse for privacy (since more pepole know what old addresses you had).
- As soon as we encrypt read receipts, sending a read receipt will be enough to tell a lot of people that you transitioned
- AEAP will make the problem of inconsistent group state worse, both because it doesn't work if the message is unencrypted (even if the design allowed it, it would be problematic security-wise) and because some chat partners may have gotten the transition and some not. We should do something against this at some point in the future, like asking the user whether they want to add/remove the members to restore consistent group state.

#### Downsides of this design:
- Inconsistent group state: Suppose Alice does an AEAP transition and sends a 1:1 message to Bob, so Bob rewrites Alice's contact. Alice, Bob and Charlie are together in a group. Before Alice writes to this group, Bob and Charlie will have different membership lists, and Bob will send messages to Alice's new address, while Charlie will send them to her old address.

#### Upsides:
- With this approach, it's easy to switch to a model where the info about the transition is encoded in the PGP key. Since the key is gossiped, the information about the transition will spread virally.
- Faster transition: If you send a message to e.g. "Delta Chat Dev", all members of the "sub-group" "delta android" will know of your transition.

### Alternatives and old discussions/plans:

- Change the contact instead of rewriting the group member lists. This seems to call for more trouble since we will end up with multiple contacts having the same email address.

- If needed, we could add a header a) indicating that the sender did an address transition or b) listing all the secondary (old) addresses.  For now, there is no big enough benefit to warrant introducing another header and its processing on the receiver side (including all the necessary checks and handling of error cases). Instead, we only check for the `Chat-Version` header to prevent accidental transitions when an MUA user sends a message from another email address with the same key.

- The condition for a transition temporarily was:

  > When receiving a message: If we are going to assign a message to a chat, but the sender is not a member of this chat\
  > AND the signing key is the same as the direct (non-gossiped) key of one of the chat members\
  > AND ...

  However, this would mean that in 1:1 messages can't trigger a transition, since we don't assign private messages to the parent chat, but always to the 1:1 chat with the sender.
  
<details>
<summary>Some previous state of the discussion, which temporarily lived in an issue description</summary>
Summarizing the discussions from https://github.com/deltachat/deltachat-core-rust/pull/2896, mostly quoting @hpk42:

1. (DONE) At the time of configure we push the current primary to become a secondary. 

2. When a message is sent out to a chat, and the message is encrypted, and we have secondary addresses, then we 
  a) add a protected "AEAP-Replacement" header that contains all secondary addresses 
  b) if any of the secondary addresses is in the chat's member list, we remove it and leave a system message that we did so
3. When an encrypted message with a replacement header is received, replace the e-mail address of all secondary contacts (if they exist) with the new primary and drop a sysmessage in all chats the secondary is member off.  This might (in edge cases) result in chats that have two or more contacts with the same e-mail address.  We might ignore this for a first release and just log a warning.  Let's maybe not get hung up on this case before everything else works. 

Notes: 
- for now we will send out aeap replacement headers forever, there is no termination condition other than lack of secondary addresses.  I think that's fine for now.  Later on we might introduce options to remove secondary addresses but i wouldn't do this for a first release/PR. 
- the design is resilient against changing e-mail providers from A to B to C and then back to A, with partially updated chats and diverging views from recipients/contacts on this transition.  In the end, you will have a primary and some secondaries, and when you start sending out messages everybody will eventually synchronize when they receive the current state of primaries/secondaries. 
- of course on incoming message for need to check for each stated secondary address in the replacement header that it uses the same signature as the signature we verified as valid with the incoming message  **-->  Also we have to somehow make sure that the signing key was not just gossiped from some random other person in some group.**
- there are no extra flags/columns in the database needed (i hope) 

#### Downsides of the chosen approach:
- Inconsistent group state: Suppose Alice does an AEAP transition and sends a 1:1 message to Bob, so Bob rewrites Alice's contact. Alice, Bob and Charlie are together in a group. Before Alice writes to this group, Bob and Charlie will have different membership lists, and Bob will send messages to Alice's new address, while Charlie will send them to her old address.
- There will be multiple contacts with the same address in the database. We will have to do something against this at some point.

The most obvious alternative would be to create a new contact with the new address and replace the old contact in the groups.

#### Upsides:
- With this approach, it's easier to switch to a model where the info about the transition is encoded in the PGP key. Since the key is gossiped, the information about the transition will spread virally.
- (Also, less important: Slightly faster transition: If you send a message to e.g. "Delta Chat Dev", all members of the "sub-group" "delta android" will know of your transition.)
- It's easier to implement (if too many problems turn up, we can still switch to another approach and didn't waste that much development time.)

[full messages](https://github.com/deltachat/deltachat-core-rust/pull/2896#discussion_r852002161)
  
_end of the previous state of the discussion_  

</details>
  
Other
-----

- The user is responsible that messages to the old address arrive at the new address, for example by configuring the old provider to forward all emails to the new one.

Notes during implementing
========================

- As far as I understand the code, unencrypted messages are unsigned. So, the transition only works if both sides have the other side's key.
