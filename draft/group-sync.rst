
Problem: missing eventual group consistency
--------------------------------------------

If group members are concurrently adding new members,
the new members will miss each other's additions, example:

1. Alice and Bob are in a two-member group

2. Then Alice adds Carol, while concurrently Bob adds Doris

Right now, the group has inconsistent memberships:

- Alice and Carol see a (Alice, Carol, Bob) group

- Bob and Doris see a (Bob, Doris, Alice)

This then leads to "sender is unknown" messages in the chat,
for example when Alice receives a message from Doris,
or when Bob receives a message from Carol.

There are also other sources for group membership inconsistency:

- leaving/deleting/adding in larger groups, while being offline,
  increases chances for inconsistent group membership

- dropped group-membership messages

- group-membership messages landing in "Spam"


Note that all these problems (can) also happen with verified groups,
then raising "false alarms" which could lure people to ignore such issues.

IOW, it's clear we need to do something about it to improve overall
reliability in group-settings.



Solution: replay group modification messages on inconsistencies
------------------------------------------------------------------

For brevity let's abbreviate "group membership modification" as **GMM**.

Delta chat has explicit GMM messages, typically encrypted to the group members
as seen by the device that sends the GMM. The [Spec](https://github.com/deltachat/deltachat-core-rust/blob/master/spec.md#add-and-remove-members) details the Mime headers and format.

If we detect membership inconsistencies we can resend relevant GMM messages
to the respective chat.  The receiving devices can process those GMM messages
as if it would be an incoming message. If for example they have already seen
the Message-ID of the GMM message, they will ignore the message. It's
probably useful to record GMM message in their original MIME-format and
not invent a new recording format. Few notes on three aspects:

- **group-membership-tracking**: All valid GMM messages are persisted in
  their full raw (signed/encrypted?) MIME-format in the DB. Note that GMM messages
  already are in the msgs table, and there is a mime_header column which we could
  extend to contain the raw Mime GMM message.

- **consistency_checking**: If an incoming GMM has a member list which is
  not consistent with our own view, broadcast a "Group-Member-Correction"
  message to all members containing a multipart list of GMMs.

- **correcting_memberships**: Upon receiving a Group-Member-Correction
  message we pass the contained GMMs to the "incoming mail pipeline"
  (without **consistency_checking** them, to avoid recursion issues)


Alice/Carol and Bob/Doris getting on the same page
++++++++++++++++++++++++++++++++++++++++++++++++++

Recall that Alice/Carol and Bob/Doris had a differening view of
group membership. With the proposed solution, when Bob receives
Alice's "Carol added" message, he will notice that Alice (and thus
also carol) did not know about Doris.  Bob's device sends a
"Chat-Group-Member-Correction" message containing his own GMM
when adding Doris. Therefore, the group's membership is healed
for everyone in a single broadcast message.

Alice might also send a Group-member-Correction message,
so there is a second chance that the group gets to know all GMMs.

Note, for example, that if for some reason Bobs and Carols provider
drop GMM messages between them (spam) that Alice and Doris can heal
it by resending GMM messages whenever they detect them to be out of sync.


Discussions of variants
++++++++++++++++++++++++

- instead of acting on GMM messages we could send corrections
  for any received message that addresses inconsistent group members but
  a) this could delay group-membership healing
  b) could lead to a lot of members sending corrections
  c) means we might rely on "To-Addresses" which we also like to strike
     at least for protected chats.

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


