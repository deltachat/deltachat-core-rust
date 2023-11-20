Running Tests
=============

Recommended way to run tests is using `scripts/run-python-test.sh`
script provided in the core repository.

This script compiles the library in debug mode and runs the tests using `tox`_.
By default it will run all "offline" tests and skip all functional
end-to-end tests that require accounts on real email servers.

.. _`tox`: https://tox.wiki
.. _livetests:

Running "Live" Tests With Temporary Accounts
--------------------------------------------

If you want to run live functional tests
you can set ``CHATMAIL_DOMAIN`` to a domain of the email server
that creates email accounts like this::

    export CHATMAIL_DOMAIN=nine.testrun.org

With this account-creation setting, pytest runs create ephemeral email accounts on the server.
These accounts have the pattern `ci-{6 characters}@{CHATMAIL_DOMAIN}`.
After setting the variable, either rerun `scripts/run-python-test.sh`
or run offline and online tests with `tox` directly::

    tox -e py

Each test run creates new accounts.

Developing the Bindings
-----------------------

If you want to develop or debug the bindings,
you can create a testing development environment using `tox`::

    export DCC_RS_DEV="$PWD"
    export DCC_RS_TARGET=debug
    tox -c python --devenv env -e py
    . env/bin/activate

Inside this environment the bindings are installed
in editable mode (as if installed with `python -m pip install -e`)
together with the testing dependencies like `pytest` and its plugins.

You can then edit the source code in the development tree
and quickly run `pytest` manually without waiting  for `tox`
to recreating the virtual environment each time.
