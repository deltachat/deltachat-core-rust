
Implementing Plugin Hooks
==========================

The Delta Chat Python bindings use `pluggy <https://pluggy.readthedocs.io>`_
for managing global and per-account plugin registration, and performing
hook calls. There are two kinds of plugins:

- Global plugins that are active for all accounts; they can implement
  hooks at account-creation and account-shutdown time.

- Account plugins that are only active during the lifetime of a
  single Account instance.


Registering a plugin
--------------------

.. autofunction:: deltachat.register_global_plugin
    :noindex:

.. automethod:: deltachat.account.Account.add_account_plugin
    :noindex:


Per-Account Hook specifications
-------------------------------

.. autoclass:: deltachat.hookspec.PerAccount
    :members:


Global Hook specifications
--------------------------

.. autoclass:: deltachat.hookspec.Global
    :members:

