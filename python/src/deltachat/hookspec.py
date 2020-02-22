""" Hooks for Python bindings to Delta Chat Core Rust CFFI"""

import pluggy


_account_name = "deltachat-account"
account_hookspec = pluggy.HookspecMarker(_account_name)
account_hookimpl = pluggy.HookimplMarker(_account_name)

_global_name = "deltachat-global"
global_hookspec = pluggy.HookspecMarker(_global_name)
global_hookimpl = pluggy.HookimplMarker(_global_name)


class PerAccount:
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

    @account_hookspec
    def log_line(self, message):
        """ log a message related to the account. """

    @account_hookspec
    def configure_completed(self, success):
        """ Called when a configure process completed. """



class Global:
    """ global hook specifications using a per-process singleton plugin manager instance.

    """
    _plugin_manager = None

    @classmethod
    def _get_plugin_manager(cls):
        if cls._plugin_manager is None:
            cls._plugin_manager = pm = pluggy.PluginManager(_global_name)
            pm.add_hookspecs(cls)
        return cls._plugin_manager

    @global_hookspec
    def at_account_init(self, account):
        """ called when `Account::__init__()` function starts executing. """
