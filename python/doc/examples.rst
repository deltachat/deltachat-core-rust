
examples
========

Once you have :doc:`installed deltachat bindings <install>`
you need email/password credentials for an IMAP/SMTP account.
Delta Chat developers and the CI system use a special URL to create
temporary e-mail accounts on [testrun.org](https://testrun.org) for testing.

Receiving a Chat message from the command line
----------------------------------------------

Here is a simple bot that:

- receives a message and sends back ("echoes") a message

- terminates the bot if the message `/quit` is sent

.. include:: ../examples/echo_and_quit.py
    :literal:

With this file in your working directory you can run the bot
by specifying a database path, an e-mail address and password of
a SMTP-IMAP account::

    $ cd examples
    $ python echo_and_quit.py /tmp/db --email ADDRESS --password PASSWORD

While this process is running you can start sending chat messages
to `ADDRESS`.

Track member additions and removals in a group
----------------------------------------------

Here is a simple bot that:

- echoes messages sent to it

- tracks if configuration completed

- tracks member additions and removals for all chat groups

.. include:: ../examples/group_tracking.py
    :literal:

With this file in your working directory you can run the bot
by specifying a database path, an e-mail address and password of
a SMTP-IMAP account::

    python group_tracking.py --email ADDRESS --password PASSWORD /tmp/db

When this process is running you can start sending chat messages
to `ADDRESS`.

Writing bots for real
-------------------------

The `deltabot repository <https://github.com/deltachat/deltabot#deltachat-example-bot>`_
contains a little framework for writing deltachat bots in Python.

