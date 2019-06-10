=========================
deltachat python bindings
=========================

This package provides bindings to the deltachat-core_ Rust -library
which provides imap/smtp/crypto handling as well as chat/group/messages
handling to Android, Desktop and IO user interfaces.

Installing pre-built packages (linux-only)
==========================================

If you have a linux system you may install the ``deltachat`` binary "wheel" package
without any "build-from-source" steps.

1. `Install virtualenv <https://virtualenv.pypa.io/en/stable/installation/>`_,
   then create a fresh python environment and activate it in your shell::

        virtualenv -p python3 venv
        source venv/bin/activate

   Afterwards, invoking ``python`` or ``pip install`` will only
   modify files in your ``venv`` directory and leave your system installation
   alone.

2. Install the wheel for linux::

        pip install deltachat

    Verify it worked by typing::

        python -c "import deltachat"


Installing a wheel from a PR/branch
---------------------------------------

For Linux, we automatically build wheels for all github PR branches
and push them to a python package index. To install the latest github ``master`` branch::

    pip install -i https://m.devpi.net/dc/master deltachat


Installing bindings from source
===============================

If you can't use "binary" method above then you need to compile
to core deltachat library::

    git clone https://github.com/deltachat/deltachat-core-rust
    cd deltachat-core-rust
    cargo build -p deltachat_ffi --release

This will result in a ``libdeltachat.so`` and ``libdeltachat.a`` files
in the ``target/release`` directory. These files are needed for
creating the python bindings for deltachat::

    cd python
    DCC_RS_DEV=`pwd`/.. pip install -e .

Now test if the bindings find the correct library::

    python -c 'import deltachat ; print(deltachat.__version__)'

This should print your deltachat bindings version.

.. note::

    If you can help to automate the building of wheels for Mac or Windows,
    that'd be much appreciated! please then get
    `in contact with us <https://delta.chat/en/contribute>`_.

Using a system-installed deltachat-core-rust
--------------------------------------------

When calling ``pip`` without specifying the ``DCC_RS_DEV`` environment
variable cffi will try to use a ``deltachat.h`` from a system location
like ``/usr/local/include`` and will try to dynamically link against a
``libdeltachat.so`` in a similar location (e.g. ``/usr/local/lib``).


Code examples
=============

You may look at `examples <https://py.delta.chat/examples.html>`_.


Running tests
=============

Get a checkout of the `deltachat-core-rust github repository`_ and type::

    pip install tox
    ./run-integration-tests.sh

If you want to run functional tests with real
e-mail test accounts, generate a "liveconfig" file where each
lines contains test account settings, for example::

    # 'liveconfig' file specifying imap/smtp accounts
    addr=some-email@example.org mail_pw=password
    addr=other-email@example.org mail_pw=otherpassword

The "keyword=value" style allows to specify any
`deltachat account config setting <https://c.delta.chat/classdc__context__t.html#aff3b894f6cfca46cab5248fdffdf083d>`_ so you can also specify smtp or imap servers, ports, ssl modes etc.
Typically DC's automatic configuration allows to not specify these settings.

The ``run-integration-tests.sh`` script will automatically use
``python/liveconfig`` if it exists, to manually run tests with this
``liveconfig`` file use::

    tox -- --liveconfig liveconfig


.. _`deltachat-core-rust github repository`: https://github.com/deltachat/deltachat-core-rust
.. _`deltachat-core`: https://github.com/deltachat/deltachat-core-rust

Running test using a debug build
--------------------------------

If you need to examine e.g. a coredump you may want to run the tests
using a debug build::

   DCC_RS_TARGET=debug ./run-integration-tests.sh -e py37 -- -x -v -k failing_test


Building manylinux1 wheels
==========================

Building portable manylinux1 wheels which come with libdeltachat.so
and all it's dependencies is easy using the provided docker tooling.

using docker pull / premade images
------------------------------------

We publish a build environment under the ``deltachat/wheel`` tag so
that you can pull it from the ``hub.docker.com`` site's "deltachat"
organization::

    $ docker pull deltachat/wheel

The ``deltachat/wheel`` image can be used to build both libdeltachat.so
and the Python wheels::

    $ docker run --rm -it -v $(pwd):/io/ deltachat/wheel /io/python/wheelbuilder/build-wheels.sh

This command runs a script within the image, after mounting ``$(pwd)`` as ``/io`` within
the docker image.  The script is specified as a path within the docker image's filesystem.
The resulting wheel files will be in ``python/wheelhouse``.


Optionally build your own docker image
--------------------------------------

If you want to build your own custom docker image you can do this::

   $ cd deltachat-core # cd to deltachat-core checkout directory
   $ docker build -t deltachat/wheel python/wheelbuilder/

This will use the ``python/wheelbuilder/Dockerfile`` to build
up docker image called ``deltachat/wheel``.  You can afterwards
find it with::

   $ docker images


Troubleshooting
---------------

On more recent systems running the docker image may crash.  You can
fix this by adding ``vsyscall=emulate`` to the Linux kernel boot
arguments commandline.  E.g. on Debian you'd add this to
``GRUB_CMDLINE_LINUX_DEFAULT`` in ``/etc/default/grub``.
