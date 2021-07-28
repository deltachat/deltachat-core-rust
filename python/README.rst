=========================
deltachat python bindings
=========================

This package provides bindings to the deltachat-core_ Rust -library
which implements IMAP/SMTP/MIME/PGP e-mail standards and offers
a low-level Chat/Contact/Message API to user interfaces and bots.


Installing pre-built packages (Linux-only)
========================================================

If you have a Linux system you may try to install the ``deltachat`` binary "wheel" packages
without any "build-from-source" steps.
Otherwise you need to `compile the Delta Chat bindings yourself <#sourceinstall>`_.

We recommend to first `install virtualenv <https://virtualenv.pypa.io/en/stable/installation.html>`_,
then create a fresh Python virtual environment and activate it in your shell::

        virtualenv venv  # or: python -m venv
        source venv/bin/activate

Afterwards, invoking ``python`` or ``pip install`` only
modifies files in your ``venv`` directory and leaves
your system installation alone.

For Linux, we automatically build wheels for all github PR branches
and push them to a python package index. To install the latest
github ``master`` branch::

    pip install --pre -i https://m.devpi.net/dc/master deltachat

To verify it worked::

    python -c "import deltachat"

.. note::

    If you can help to automate the building of wheels for Mac or Windows,
    that'd be much appreciated! please then get
    `in contact with us <https://delta.chat/en/contribute>`_.


Running tests
=============

After successful binding installation you can install a few more
Python packages before running the tests::

    python -m pip install pytest pytest-xdist pytest-timeout pytest-rerunfailures requests
    pytest -v tests

This will run all "offline" tests and skip all functional
end-to-end tests that require accounts on real e-mail servers.

.. _livetests:

running "live" tests with temporary accounts
---------------------------------------------

If you want to run live functional tests you can set ``DCC_NEW_TMP_EMAIL`` to a URL that creates e-mail accounts.  Most developers use https://testrun.org URLS created and managed by [mailadm](https://mailadm.readthedocs.io/en/latest/).

Please feel free to contact us through a github issue or by e-mail and we'll send you a URL that you can then use for functional tests like this:

    export DCC_NEW_TMP_EMAIL=<URL you got from us>

With this account-creation setting, pytest runs create ephemeral e-mail accounts on the http://testrun.org server.  These accounts exists only for one hour and then are removed completely.
One hour is enough to invoke pytest and run all offline and online tests:

    pytest

    # or if you have installed pytest-xdist for parallel test execution
    pytest -n6

Each test run creates new accounts.


.. _sourceinstall:

Installing bindings from source (Updated: July 2020)
=========================================================

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
virtual environment and activate it in your shell::

   cd python
   python3 -m venv venv  # or: virtualenv venv
   source venv/bin/activate

You should now be able to build the python bindings using the supplied script::

   python install_python_bindings.py

The core compilation and bindings building might take a while,
depending on the speed of your machine.
The bindings will be installed in release mode but with debug symbols.
The release mode is currently necessary because some tests generate RSA keys
which is prohibitively slow in non-release mode.


Code examples
=============

You may look at `examples <https://py.delta.chat/examples.html>`_.


.. _`deltachat-core-rust github repository`: https://github.com/deltachat/deltachat-core-rust
.. _`deltachat-core`: https://github.com/deltachat/deltachat-core-rust


Building manylinux based wheels
====================================

Building portable manylinux wheels which come with libdeltachat.so
can be done with docker-tooling.

using docker pull / premade images
------------------------------------

We publish a build environment under the ``deltachat/coredeps`` tag so
that you can pull it from the ``hub.docker.com`` site's "deltachat"
organization::

    $ docker pull deltachat/coredeps

This docker image can be used to run tests and build Python wheels for all interpreters::

    $ docker run -e DCC_NEW_TMP_EMAIL \
       --rm -it -v \$(pwd):/mnt -w /mnt \
       deltachat/coredeps scripts/run_all.sh


Optionally build your own docker image
--------------------------------------

If you want to build your own custom docker image you can do this::

   $ cd deltachat-core # cd to deltachat-core checkout directory
   $ docker build -t deltachat/coredeps scripts/docker_coredeps

This will use the ``scripts/docker_coredeps/Dockerfile`` to build
up docker image called ``deltachat/coredeps``.  You can afterwards
find it with::

   $ docker images


Troubleshooting
---------------

On more recent systems running the docker image may crash.  You can
fix this by adding ``vsyscall=emulate`` to the Linux kernel boot
arguments commandline.  E.g. on Debian you'd add this to
``GRUB_CMDLINE_LINUX_DEFAULT`` in ``/etc/default/grub``.
