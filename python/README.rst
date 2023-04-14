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

If you have a Linux system you may install the ``deltachat`` binary "wheel" packages
without any "build-from-source" steps.
Otherwise you need to `compile the Delta Chat bindings yourself`__.

__ sourceinstall_

We recommend to first create a fresh Python virtual environment
and activate it in your shell::

    python -m venv env
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

Recommended way to run tests is using `scripts/run-python-test.sh`
script provided in the core repository.

This script compiles the library in debug mode and runs the tests using `tox`_.
By default it will run all "offline" tests and skip all functional
end-to-end tests that require accounts on real e-mail servers.

.. _`tox`: https://tox.wiki
.. _livetests:

Running "live" tests with temporary accounts
--------------------------------------------

If you want to run live functional tests you can set ``DCC_NEW_TMP_EMAIL`` to a URL that creates e-mail accounts.  Most developers use https://testrun.org URLs created and managed by `mailadm <https://mailadm.readthedocs.io/>`_.

Please feel free to contact us through a github issue or by e-mail and we'll send you a URL that you can then use for functional tests like this::

    export DCC_NEW_TMP_EMAIL=<URL you got from us>

With this account-creation setting, pytest runs create ephemeral e-mail accounts on the http://testrun.org server.
These accounts are removed automatically as they expire.
After setting the variable, either rerun `scripts/run-python-test.sh`
or run offline and online tests with `tox` directly::

    tox -e py

Each test run creates new accounts.

Developing the bindings
-----------------------

If you want to develop or debug the bindings,
you can create a testing development environment using `tox`::

    tox -c python --devenv env
    . env/bin/activate

Inside this environment the bindings are installed
in editable mode (as if installed with `python -m pip install -e`)
together with the testing dependencies like `pytest` and its plugins.

You can then edit the source code in the development tree
and quickly run `pytest` manually without waiting  for `tox`
to recreating the virtual environment each time.

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

First, build the core library::

   cargo build --release -p deltachat_ffi --features jsonrpc

`jsonrpc` feature is required even if not used by the bindings
because `deltachat.h` includes JSON-RPC functions unconditionally.

Create the virtual environment and activate it:

   python -m venv env
   source env/bin/activate

Build and install the bindings:

   export DCC_RS_DEV="$PWD"
   export DCC_RS_TARGET=release
   python -m pip install ./python

`DCC_RS_DEV` environment variable specifies the location of
the core development tree. If this variable is not set,
`libdeltachat` library and `deltachat.h` header are expected
to be installed system-wide.

When `DCC_RS_DEV` is set, `DCC_RS_TARGET` specifies
the build profile name to look up the artifacts
in the target directory.
In this case setting it can be skipped because
`DCC_RS_TARGET=release` is the default.

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
