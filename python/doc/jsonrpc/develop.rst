===========
Development
===========

To develop JSON-RPC bindings,
clone the `deltachat-core-rust <https://github.com/deltachat/deltachat-core-rust/>`_ repository::

   git clone https://github.com/deltachat/deltachat-core-rust.git

Testing
=======

To run online tests, set ``CHATMAIL_DOMAIN``
to a domain of the email server
that can be used to create testing accounts::

    export CHATMAIL_DOMAIN=nine.testrun.org

Then run ``scripts/run-rpc-test.sh``
to build debug version of ``deltachat-rpc-server``
and run ``deltachat-rpc-client`` tests
in a separate virtual environment managed by `tox <https://tox.wiki/>`_.

Development Environment
=======================

Creating a new virtual environment
to run the tests each time
as ``scripts/run-rpc-test.sh`` does is slow
if you are changing the tests or the code
and want to rerun the tests each time.

If you are developing the tests,
it is better to create a persistent virtual environment.
You can do this by running ``scripts/make-rpc-testenv.sh``.
This creates a virtual environment ``venv`` which you can then enter with::

   . venv/bin/activate

Then you can run the tests with

::

    pytest deltachat-rpc-client/tests/

Refer to `pytest documentation <https://docs.pytest.org/>` for details.

If make the changes to Delta Chat core
or Python bindings, you can rebuild the environment by rerunning
``scripts/make-rpc-testenv.sh``.
It is ok to rebuild the activated environment this way,
you do not need to deactivate or reactivate the environment each time.

Using REPL
==========

Once you have a development environment,
you can quickly test things in REPL::

   $ python
   >>> from deltachat_rpc_client import *
   >>> rpc = Rpc()
   >>> rpc.start()
   >>> dc = DeltaChat(rpc)
   >>> system_info = dc.get_system_info()
   >>> system_info["level"]
   'awesome'
   >>> rpc.close()
