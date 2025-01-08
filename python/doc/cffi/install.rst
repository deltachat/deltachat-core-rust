Install
=======

Installing pre-built packages (Linux-only)
------------------------------------------

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

.. _sourceinstall:

Installing bindings from source
-------------------------------

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

   cargo build --release -p deltachat_ffi

Create the virtual environment and activate it::

   python -m venv env
   source env/bin/activate

Build and install the bindings::

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
