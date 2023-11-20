Building Manylinux-Based Wheels
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

    $ docker run -e CHATMAIL_DOMAIN \
       --rm -it -v $(pwd):/mnt -w /mnt \
       deltachat/coredeps scripts/run_all.sh
