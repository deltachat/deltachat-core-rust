
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

- All Chat-Group-Member-Added/Removed messages are recorded in their
  full raw (signed and encrypted) mime-format in the DB

- If an incoming member-add/member-delete messages has a member list
  which is, apart from the added/removed member, not consistent
  with our own view, send a "Chat-Group-Member-Correction" message out,
  attaching the original added/removed mime-message for all mismatching contacts.

- When receiving added/removed attachments don't do the
  check_consistency+correction message dance.  This avoids recursion problems
  and hard-to-reason-about chatter.

Notes:

- mechanism works for both encrypted and unencrypted add/del messages

- we already have a "mime_headers" column in the DB for each incoming message.
  We could extend it to also include the payload and store mime unconditionally
  for member-added/removed messages.

- multiple member-added/removed messages can be attached in a single message

- is minimal on the number of overall messages to reach group consistency
  (best-case: no extra messages, the ABCD case above: max two extra messages)

- somewhat backward compatible: older clients will probably ignore
  messages which are signed by someone who is not the outer From-address.

- we can quite easily extend the mechanism to also provide the group-avatar or
  other meta-information.




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


