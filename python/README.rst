=========================
deltachat python bindings
=========================

This package provides bindings to the deltachat-core_ Rust -library
which implements IMAP/SMTP/MIME/PGP e-mail standards and offers
a low-level Chat/Contact/Message API to user interfaces and bots.


Installing bindings from source (Updated: 20-Jan-2020)
=========================================================

Install Rust and Cargo first.  Deltachat needs a specific nightly
version, the easiest is probably to first install Rust stable from
rustup and then use this to install the correct nightly version.

Bootstrap Rust and Cargo by using rustup::

   curl https://sh.rustup.rs -sSf | sh

Then GIT clone the deltachat-core-rust repo and get the actual
rust- and cargo-toolchain needed by deltachat::

   git clone https://github.com/deltachat/deltachat-core-rust
   cd deltachat-core-rust
   rustup show

To install the Delta Chat Python bindings make sure you have Python3 installed.
E.g. on Debian-based systems `apt install python3 python3-pip
python3-venv` should give you a usable python installation.

Ensure you are in the deltachat-core-rust/python directory, create the
virtual environment and activate it in your shell::

   cd python
   python3 -m venv venv  # or: virtualenv venv
   source venv/bin/activate

You should now be able to build the python bindings using the supplied script::

   ./install_python_bindings.py

The installation might take a while, depending on your machine.
The bindings will be installed in release mode but with debug symbols.
The release mode is currently necessary because some tests generate RSA keys
which is prohibitively slow in non-release mode.

After successful binding installation you can install a few more
Python packages before running the tests::

    python -m pip install pytest pytest-timeout pytest-rerunfailures requests
    pytest -v tests


running "live" tests (experimental)
-----------------------------------

If you want to run "liveconfig" functional tests you can set
``DCC_PY_LIVECONFIG`` to:

- a particular https-url that you can ask for from the delta
  chat devs.

- or the path of a file that contains two lines, each describing
  via "addr=... mail_pw=..." a test account login that will
  be used for the live tests.

With ``DCC_PY_LIVECONFIG`` set pytest invocations will use real
e-mail accounts and run through all functional "liveconfig" tests.


Installing pre-built packages (Linux-only)
========================================================

If you have a Linux system you may try to install the ``deltachat`` binary "wheel" packages
without any "build-from-source" steps.

We suggest to `Install virtualenv <https://virtualenv.pypa.io/en/stable/installation/>`_,
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


Code examples
=============

You may look at `examples <https://py.delta.chat/examples.html>`_.


.. _`deltachat-core-rust github repository`: https://github.com/deltachat/deltachat-core-rust
.. _`deltachat-core`: https://github.com/deltachat/deltachat-core-rust


Building manylinux1 based wheels
================================

Building portable manylinux1 wheels which come with libdeltachat.so
can be done with docker-tooling.

using docker pull / premade images
------------------------------------

We publish a build environment under the ``deltachat/coredeps`` tag so
that you can pull it from the ``hub.docker.com`` site's "deltachat"
organization::

    $ docker pull deltachat/coredeps

This docker image can be used to run tests and build Python wheels for all interpreters::

    $ docker run -e DCC_PY_LIVECONFIG \
       --rm -it -v \$(pwd):/mnt -w /mnt \
       deltachat/coredeps ci_scripts/run_all.sh


Optionally build your own docker image
--------------------------------------

If you want to build your own custom docker image you can do this::

   $ cd deltachat-core # cd to deltachat-core checkout directory
   $ docker build -t deltachat/coredeps ci_scripts/docker_coredeps

This will use the ``ci_scripts/docker_coredeps/Dockerfile`` to build
up docker image called ``deltachat/coredeps``.  You can afterwards
find it with::

   $ docker images


Troubleshooting
---------------

On more recent systems running the docker image may crash.  You can
fix this by adding ``vsyscall=emulate`` to the Linux kernel boot
arguments commandline.  E.g. on Debian you'd add this to
``GRUB_CMDLINE_LINUX_DEFAULT`` in ``/etc/default/grub``.
