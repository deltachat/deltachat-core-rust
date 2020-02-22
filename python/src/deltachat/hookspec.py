""" Hooks for Python bindings to Delta Chat Core Rust CFFI"""

import pluggy

__all__ = ["account_hookspec", "account_hookimpl", "AccountHookSpecs"]

_account_name = "deltachat-account"
account_hookspec = pluggy.HookspecMarker(_account_name)
account_hookimpl = pluggy.HookimplMarker(_account_name)


class AccountHookSpecs:
    """ per-Account-instance hook specifications.

    Account hook implementations need to be registered with an Account instance.
    """
    @classmethod
    def _make_plugin_manager(cls):
        pm = pluggy.PluginManager(_account_name)
        pm.add_hookspecs(cls)
        return pm

    @account_hookspec
    def process_low_level_event(self, event_name, data1, data2):
        """ process a CFFI low level events for a given account. """
