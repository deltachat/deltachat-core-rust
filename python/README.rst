=========================
DeltaChat Python bindings
=========================

This package provides `Python bindings`_ to the `deltachat-core library`_
which implements IMAP/SMTP/MIME/OpenPGP e-mail standards and offers
a low-level Chat/Contact/Message API to user interfaces and bots.

.. _`deltachat-core library`: https://github.com/deltachat/deltachat-core-rust
.. _`Python bindings`: https://py.delta.chat/

Installing pre-built packages (Linux-only)
==========================================

If you have a Linux system you may try to install the ``deltachat`` binary "wheel" packages
without any "build-from-source" steps.
Otherwise you need to `compile the Delta Chat bindings yourself`__.

__ sourceinstall_

We recommend to first `install virtualenv <https://virtualenv.pypa.io/en/stable/installation.html>`_,
then create a fresh Python virtual environment and activate it in your shell::

        virtualenv env  # or: python -m venv
        source env/bin/activate

Afterwards, invoking ``python`` or ``pip install`` only
modifies files in your ``env`` directory and leaves
your system installation alone.

For Linux we build wheels for all releases and push them to a python package
index. To install the latest release::

    pip install deltachat

To verify it worked::

    python -c "import deltachat"

Running tests
=============

Recommended way to run tests is using `tox <https://tox.wiki>`_.
After successful binding installation you can install tox
and run the tests::

    pip install tox
    tox -e py3

This will run all "offline" tests and skip all functional
end-to-end tests that require accounts on real e-mail servers.

.. _livetests:

Running "live" tests with temporary accounts
--------------------------------------------

If you want to run live functional tests you can set ``DCC_NEW_TMP_EMAIL`` to a URL that creates e-mail accounts.  Most developers use https://testrun.org URLs created and managed by `mailadm <https://mailadm.readthedocs.io/>`_.

Please feel free to contact us through a github issue or by e-mail and we'll send you a URL that you can then use for functional tests like this::

    export DCC_NEW_TMP_EMAIL=<URL you got from us>

With this account-creation setting, pytest runs create ephemeral e-mail accounts on the http://testrun.org server.  These accounts exists only for one hour and then are removed completely.
One hour is enough to invoke pytest and run all offline and online tests::

    tox -e py3

Each test run creates new accounts.

.. _sourceinstall:

Installing bindings from source
===============================

Install Rust and Cargo first.
The easiest is probably to use `rustup <https://rustup.rs/>`_.

Bootstrap Rust and Cargo by using rustup::

   curl https://sh.rustup.rs -sSf | sh

Then clone the deltachat-core-rust repo::

   git clone https://github.com/deltachat/deltachat-core-rust
   cd deltachat-core-rust

To install the Delta Chat Python bindings make sure you have Python3 installed.
E.g. on Debian-based systems `apt install python3 python3-pip
python3-venv` should give you a usable python installation.

Ensure you are in the deltachat-core-rust/python directory, create the
virtual environment with dependencies using tox
and activate it in your shell::

   cd python
   tox --devenv env
   source env/bin/activate

You should now be able to build the python bindings using the supplied script::

   python3 install_python_bindings.py

The core compilation and bindings building might take a while,
depending on the speed of your machine.

Building manylinux based wheels
===============================

Building portable manylinux wheels which come with libdeltachat.so
can be done with Docker_ or Podman_.

.. _Docker: https://www.docker.com/
.. _Podman: https://podman.io/

If you want to build your own wheels, build container image first::

   $ cd deltachat-core-rust # cd to deltachat-core-rust working tree
   $ docker build -t deltachat/coredeps scripts/coredeps

This will use the ``scripts/coredeps/Dockerfile`` to build
container image called ``deltachat/coredeps``.  You can afterwards
find it with::

   $ docker images

This docker image can be used to run tests and build Python wheels for all interpreters::

    $ docker run -e DCC_NEW_TMP_EMAIL \
       --rm -it -v $(pwd):/mnt -w /mnt \
       deltachat/coredeps scripts/run_all.sh
