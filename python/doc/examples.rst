

examples
========


Playing around on the commandline
----------------------------------

Once you have :doc:`installed deltachat bindings <install>`
you can start playing from the python interpreter commandline.
For example you can type ``python`` and then::

    # instantiate and configure deltachat account
    import deltachat
    ac = deltachat.Account("/tmp/db")

    # start configuration activity and smtp/imap threads
    ac.start_threads()
    ac.configure(addr="test2@hq5.merlinux.eu", mail_pw="********")

    # create a contact and send a message
    contact = ac.create_contact("someother@email.address")
    chat = ac.create_chat_by_contact(contact)
    chat.send_text("hi from the python interpreter command line")

Checkout our :doc:`api` for the various high-level things you can do
to send/receive messages, create contacts and chats.


Looking at a real example
-------------------------

The `deltabot repository <https://github.com/deltachat/deltabot#deltachat-example-bot>`_
contains a real-life example of Python bindings usage.


