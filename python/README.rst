=========================
deltachat python bindings
=========================

This package provides bindings to the deltachat-core_ Rust -library
which provides imap/smtp/crypto handling as well as chat/group/messages
handling to Android, Desktop and IO user interfaces.


Installing bindings from source   (Updated: 21-Dec-2019)
===============================

To compile and use the Python Bindings from source::

Lets do a quick package update (using Ubuntu 18.04 with python3.6), make sure you have proper python and pip installed too (I am using python3 and pip3)::


   apt update
   apt install python3-pip
   pip3 install pytest pytest-timeout pytest-rerunfailures requests



Install Rust and Cargo first, we are doing a NIGHTLY Rust install otherwise one of the dependency (Subtle v2.2.2) will fail, it only works with Nightly versions::

   cd 
   curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
   
GIT clone the repo and run the Python Binding installation script::

   git clone https://github.com/deltachat/deltachat-core-rust
   cd deltachat-core-rust
   cd python
   python3 install_python_bindings.py

If you still get a error saying compiling failed or !feature not supported in this release/stable, use the following command below and re-run the above command to recompile::

   rustup install nightly
   ## OR ##
   /root/.cargo/bin/rustup install nightly


The installation takes 20-60 minutes (depending on your machine). 
The bindings will be installed in release mode but with debug symbols.
The release mode is necessary because some tests generate RSA keys
which is prohibitively slow in debug mode.

After successful binding installation you can finally run the tests::

    pytest -v tests

.. note::

    Some tests are sometimes failing/hanging because of
    https://github.com/deltachat/deltachat-core-rust/issues/331
    and
    https://github.com/deltachat/deltachat-core-rust/issues/326


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


============================================================================================================================
(21-Dec-2019) THE BELOW WHEELS ARE CURRENTLY NOT WORKING/BROKEN, COMPILE FROM SOURCE USING ABOVE INSTRUCTIONS INSTEAD 
============================================================================================================================

Installing pre-built packages (linux-only)  (OUTDATED)
==========================================

If you have a linux system you may install the ``deltachat`` binary "wheel" package
without any "build-from-source" steps.

1. `Install virtualenv <https://virtualenv.pypa.io/en/stable/installation/>`_,
   then create a fresh python environment and activate it in your shell::

        virtualenv venv  # or: python -m venv
        source venv/bin/activate

   Afterwards, invoking ``python`` or ``pip install`` will only
   modify files in your ``venv`` directory and leave your system installation
   alone.

2. Install the wheel for linux::

        pip install deltachat

    Verify it worked by typing::

        python -c "import deltachat"


Installing a wheel from a PR/branch    (OUTDATED)
---------------------------------------

For Linux, we automatically build wheels for all github PR branches
and push them to a python package index. To install the latest github ``master`` branch::

    pip install -i https://m.devpi.net/dc/master deltachat

.. note::

    If you can help to automate the building of wheels for Mac or Windows,
    that'd be much appreciated! please then get
    `in contact with us <https://delta.chat/en/contribute>`_.






Code examples
=============

You may look at `examples <https://py.delta.chat/examples.html>`_.


.. _`deltachat-core-rust github repository`: https://github.com/deltachat/deltachat-core-rust
.. _`deltachat-core`: https://github.com/deltachat/deltachat-core-rust


Building manylinux1 wheels
==========================

.. note::

   This section may not fully work.

Building portable manylinux1 wheels which come with libdeltachat.so
and all it's dependencies is easy using the provided docker tooling.

using docker pull / premade images
------------------------------------

We publish a build environment under the ``deltachat/coredeps`` tag so
that you can pull it from the ``hub.docker.com`` site's "deltachat"
organization::

    $ docker pull deltachat/coredeps

This docker image can be used to run tests and build Python wheels for all interpreters::

    $ bash ci_scripts/ci_run.sh

This command runs tests and build-wheel scripts in a docker container.


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
