
Problem: missing eventual group consistency
--------------------------------------------

If group members are concurrently adding new members,
the new members will miss each other's additions, example:

- Alice and Bob are in a two-member group

- Alice adds Carol, concurrently Bob adds Doris

- Carol will see a three-member group (Alice, Bob, Carol),
  Doris will see a different three-member group (Alice, Bob, Doris),
  and only Alice and Bob will have all four members.

Note that for verified groups any mitigation mechanism likely
needs to make all clients to know who originally added a member.


solution: memorize+attach (possible encrypted) chat-meta mime messages
----------------------------------------------------------------------

For reference, please see https://github.com/deltachat/deltachat-core-rust/blob/master/spec.md#add-and-remove-members how MemberAdded/Removed messages are shaped.


- All Chat-Group-Member-Added/Removed messages are recorded in their
  full raw (signed and encrypted) mime-format in the DB

- If an incoming member-add/member-delete messages has a member list
  which is, apart from the added/removed member, not consistent
  with our own view, broadcast a "Chat-Group-Member-Correction" message to
  all members, attaching the original added/removed mime-message for all mismatching
  contacts.  If we have no relevant add/del information, don't send a
  correction message out.

- Upong receiving added/removed attachments we don't do the
  check_consistency+correction message dance.
  This avoids recursion problems and hard-to-reason-about chatter.

Notes:

- mechanism works for both encrypted and unencrypted add/del messages

- we already have a "mime_headers" column in the DB for each incoming message.
  We could extend it to also include the payload and store mime unconditionally
  for member-added/removed messages.

- multiple member-added/removed messages can be attached in a single
  correction message

- it is minimal on the number of overall messages to reach group consistency
  (best-case: no extra messages, the ABCD case above: max two extra messages)

- somewhat backward compatible: older clients will probably ignore
  messages which are signed by someone who is not the outer From-address.

- the correction-protocol also helps with dropped messages.  If a member
  did not see a member-added/removed message, the next member add/removed
  message in the group will likely heal group consistency for this member.

- we can quite easily extend the mechanism to also provide the group-avatar or
  other meta-information.

Discussions of variants
++++++++++++++++++++++++

- instead of acting on MemberAdded/Removed message we could send
  corrections for any received message that addresses inconsistent group members but
  a) this would delay group-membership healing
  b) could lead to a lot of members sending corrections

- instead of broadcasting correction messages we could only send it to
  the sender of the inconsistent member-added/removed message.
  A receiver of such a correction message would then need to forward
  the message to the members it thinks also have an inconsistent view.
  This sounds complicated and error-prone.  Concretely, if Alice
  receives Bob's "Member-added: Doris" message, then Alice
  broadcasting the correction message with "Member-added: Carol"
  would reach all four members, healing group consistency in one step.
  If Bob meanwhile receives Alice's "Member-Added: Carol" message,
  Bob would broadcast a correction message to all four members as well.
  (Imagine a situation where Alice/Bob added Carol/Doris
  while both being in an offline or bad-connection situation).


solution2: repeat member-added/removed messages
---------------------------------------------------

Introduce a new Chat-Group-Member-Changed header and deprecate Chat-Group-Member-Added/Removed
but keep sending out the old headers until the new protocol is sufficiently deployed.

The new Chat-Group-Member-Changed header contains a Time-to-Live number (TTL)
which controls repetition of the signed "add/del e-mail address" payload.

Example::

    Chat-Group-Member-Changed: TTL add "somedisplayname" someone@example.org
        owEBYQGe/pANAwACAY47A6J5t3LWAcsxYgBeTQypYWRkICJzb21lZGlzcGxheW5h
        bWUiIHNvbWVvbmVAZXhhbXBsZS5vcmcgCokBHAQAAQIABgUCXk0MqQAKCRCOOwOi
        ebdy1hfRB/wJ74tgFQulicthcv9n+ZsqzwOtBKMEVIHqJCzzDB/Hg/2z8ogYoZNR
        iUKKrv3Y1XuFvdKyOC+wC/unXAWKFHYzY6Tv6qDp6r+amt+ad+8Z02q53h9E55IP
        FUBdq2rbS8hLGjQB+mVRowYrUACrOqGgNbXMZjQfuV7fSc7y813OsCQgi3tjstup
        b+uduVzxCp3PChGhcZPs3iOGCnQvSB8VAaLGMWE2d7nTo/yMQ0Jx69x5qwfXogTk
        mTt5rOJyrosbtf09TMKFzGgtqBcEqHLp3+mQpZQ+WHUKAbsRa8Jc9DOUOSKJ8SNM
        clKdskprY+4LY0EBwLD3SQ7dPkTITCRD
        =P6GG

TTL is set to "2" on an initial Chat-Group-Member-Changed add/del message.
Receivers will apply the add/del change to the group-membership,
decrease the TTL by 1, and if TTL>0 re-sent the header.

The "add|del e-mail address" payload is pgp-signed and repeated verbatim.
This allows to propagate, in a cryptographically secured way,
who added a member. This is particularly important for allowing
to show in verified groups who added a member (planned).

Disadvantage to solution 1:

- requires to specify encoding and precise rules for what/how is signed.

- causes O(N^2) extra messages

- Not easily extendable for other things (without introducing a new
  header / encoding)


