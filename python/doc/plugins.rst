
Implementing Plugin Hooks
==========================

The Delta Chat Python bindings use `pluggy <https://pluggy.readthedocs.io>`_
for managing global and per-account plugin registration, and performing
hook calls.


Registering a plugin
--------------------

.. autoclass:: deltachat.register_global_plugin

.. autoclass:: deltachat.account.Account.add_account_plugin


Per-Account Hook specifications
-------------------------------

.. autoclass:: deltachat.hookspec.PerAccount
    :members:


Global Hook specifications
--------------------------

.. autoclass:: deltachat.hookspec.Global
    :members:

