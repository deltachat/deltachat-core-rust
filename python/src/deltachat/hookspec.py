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

    If you write a plugin you need to implement one of the following hooks.
    """
    @classmethod
    def _make_plugin_manager(cls):
        pm = pluggy.PluginManager(_account_name)
        pm.add_hookspecs(cls)
        return pm

    @account_hookspec
    def process_ffi_event(self, ffi_event):
        """ process a CFFI low level events for a given account.

        ffi_event has "name", "data1", "data2" values as specified
        with `DC_EVENT_* <https://c.delta.chat/group__DC__EVENT.html>`_.
        """

    @account_hookspec
    def log_line(self, message):
        """ log a message related to the account. """

    @account_hookspec
    def configure_completed(self, success):
        """ Called when a configure process completed. """

    @account_hookspec
    def process_incoming_message(self, message):
        """ Called on any incoming message (to deaddrop or chat). """

    @account_hookspec
    def process_message_delivered(self, message):
        """ Called when an outgoing message has been delivered to SMTP. """

    @account_hookspec
    def member_added(self, chat, contact):
        """ Called for each contact added to a chat. """


class Global:
    """ global hook specifications using a per-process singleton
    plugin manager instance.

    """
    _plugin_manager = None

    @classmethod
    def _get_plugin_manager(cls):
        if cls._plugin_manager is None:
            cls._plugin_manager = pm = pluggy.PluginManager(_global_name)
            pm.add_hookspecs(cls)
        return cls._plugin_manager

    @global_hookspec
    def account_init(self, account):
        """ called when `Account::__init__()` function starts executing. """

    @global_hookspec
    def account_after_shutdown(self, account, dc_context):
        """ Called after the account has been shutdown. """
