
simplify/streamline mark-seen/delete/move/send-mdn job handling
---------------------------------------------------------------

Idea: Introduce a new "msgwork" sql table that looks very
much like the jobs table but has a primary key "msgid"
and no job id and no foreign-id anymore. This opens up
bulk-processing by looking at the whole table and combining
flag-setting to reduce imap-roundtrips and select-folder calls.

Concretely, these IMAP jobs:

    DeleteMsgOnImap
    MarkseenMsgOnImap
    MoveMsg

Would be replaced by a few per-message columns in the new msgwork table:

- needs_mark_seen: (bool) message shall be marked as seen on imap
- needs_to_move: (bool) message should be moved to mvbox_folder
- deletion_time: (target_time or 0) message shall be deleted at specified time
- needs_send_mdn: (bool) MDN shall be sent

The various places that currently add the (replaced) jobs
would now add/modify the respective message record in the message-work table.

Looking at a single message-work entry conceptually looks like this::

    if msg.server_uid==0:
        return RetryLater  # nothing can be done without server_uid

    if msg.deletion_time > current_time:
        imap.mark_delete(msg)  # might trigger early exit with a RetryLater/Failed
        clear(needs_deletion)
        clear(mark_seen)

    if needs_mark_seen:
        imap.mark_seen(msg)    # might trigger early exit with a RetryLater/Failed
        clear(needs_mark_seen)

    if needs_send_mdn:
        schedule_smtp_send_mdn(msg)
        clear(needs_send_mdn)

    if any_flag_set():
        retrylater
    # remove msgwork entry from table


Notes/Questions:

- it's unclear how much we need per-message retry-time tracking/backoff

- drafting bulk processing algo is useful before
  going for the implementation, i.e. including select_folder calls etc.

- maybe it's better to not have bools for the flags but

  0 (no change)
  1 (set the imap flag)
  2 (clear the imap flag)

  and design such that we can cover all imap flags.

- It might not be necessary to keep needs_send_mdn state in this table
  if this can be decided rather when we succeed with mark_seen/mark_delete.
