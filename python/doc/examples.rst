

examples
========


Sending a Chat message from the command line
---------------------------------------------

Once you have :doc:`installed deltachat bindings <install>`
you can start playing from the python interpreter commandline.
For example you can type ``python`` and then::

    # instantiate and configure deltachat account
    import deltachat
    ac = deltachat.Account("/tmp/db")
    ac.set_config("addr", "address@example.org")
    ac.set_config("mail_pwd", "some password")

    # start IO threads and perform configuration
    ac.start()

    # create a contact and send a message
    contact = ac.create_contact("someother@email.address")
    chat = ac.create_chat_by_contact(contact)
    chat.send_text("hi from the python interpreter command line")

    # shutdown IO threads
    ac.shutdown()


Checkout our :doc:`api` for the various high-level things you can do
to send/receive messages, create contacts and chats.


Receiving a Chat message from the command line
----------------------------------------------

Instantiate an account and register a plugin to process
incoming messages:

.. include:: ../examples/echo_and_quit.py
    :literal:

Checkout our :doc:`api` for the various high-level things you can do
to send/receive messages, create contacts and chats.


Looking at a real example
-------------------------

The `deltabot repository <https://github.com/deltachat/deltabot#deltachat-example-bot>`_
contains a real-life example of Python bindings usage.


