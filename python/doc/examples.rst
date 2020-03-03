

examples
========


Receiving a Chat message from the command line
----------------------------------------------

Once you have :doc:`installed deltachat bindings <install>`
you can start playing from the python interpreter commandline.

Here is a simple module that implements a bot that:

- receives a message and sends back an "echo" message

- terminates the bot if the message `/quit` is sent

.. include:: ../examples/echo_and_quit.py
    :literal:

With this file in your working directory you can run the bot
by specifying a database path, an e-mail address and password of
a SMTP-IMAP account::

    python echo_and_quit.py --db /tmp/db --email ADDRESS --password PASSWORD

While this process is running you can start sending chat messages
to `ADDRESS`.

Writing bots for real
-------------------------

The `deltabot repository <https://github.com/deltachat/deltabot#deltachat-example-bot>`_
contains a little framework for writing deltachat bots in Python.

