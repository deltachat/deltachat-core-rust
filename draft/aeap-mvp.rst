AEAP MVP
========

Changes to the UIs
------------------

- The secondary self addresses (see below) are shown in the UI, but not editable.

- When the user changed the email address in the configure screen, show a dialog to the user, either directly explaining things or with a link to the FAQ (see "Other" below)

Changes in the core
-------------------

- We have one primary self address and any number of secondary self addresses. `is_self_addr()` checks all of them.

- If the user does a reconfigure and changes the email address, the previous address is added as a secondary self address.

  - don't forget to deduplicate secondary self addresses in case the user switches back and forth between addresses).

  - The key stays the same.

- No changes for 1:1 chats, there simply is a new one

- When we send a message to a group, and the primary address is not a member of a group, but a secondary address is:
  
  Add Chat-Group-Member-Removed=<old address> and Chat-Group-Member-Added=<new address> headers to this message

  - On the receiving side, make sure that we accept this (even in verified groups) if the message is signed and the key stayed the same

Other
-----

- The user is responsible that messages to the old address arrive at the new address, for example by configuring the old provider to forward all emails to the new one.
