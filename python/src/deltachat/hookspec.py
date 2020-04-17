""" Hooks for Python bindings to Delta Chat Core Rust CFFI"""

import pluggy


account_spec_name = "deltachat-account"
account_hookspec = pluggy.HookspecMarker(account_spec_name)
account_hookimpl = pluggy.HookimplMarker(account_spec_name)

global_spec_name = "deltachat-global"
global_hookspec = pluggy.HookspecMarker(global_spec_name)
global_hookimpl = pluggy.HookimplMarker(global_spec_name)


class PerAccount:
    """ per-Account-instance hook specifications.

    Except for ac_process_ffi_event all hooks are executed
    in the thread which calls Account.wait_shutdown().
    """
    @classmethod
    def _make_plugin_manager(cls):
        pm = pluggy.PluginManager(account_spec_name)
        pm.add_hookspecs(cls)
        return pm

    @account_hookspec
    def ac_process_ffi_event(self, ffi_event):
        """ process a CFFI low level events for a given account.

        ffi_event has "name", "data1", "data2" values as specified
        with `DC_EVENT_* <https://c.delta.chat/group__DC__EVENT.html>`_.

        DANGER: this hook is executed from the callback invoked by core.
        Hook implementations need to be short running and can typically
        not call back into core because this would easily cause recursion issues.
        """

    @account_hookspec
    def ac_log_line(self, message):
        """ log a message related to the account. """

    @account_hookspec
    def ac_configure_completed(self, success):
        """ Called when a configure process completed. """

    @account_hookspec
    def ac_incoming_message(self, message):
        """ Called on any incoming message (to deaddrop or chat). """

    @account_hookspec
    def ac_message_delivered(self, message):
        """ Called when an outgoing message has been delivered to SMTP. """

    @account_hookspec
    def ac_chat_modified(self, chat):
        """ Chat was created or modified regarding membership, avatar, title. """

    @account_hookspec
    def ac_member_added(self, chat, contact, sender):
        """ Called for each contact added to an accepted chat. """

    @account_hookspec
    def ac_member_removed(self, chat, contact, sender):
        """ Called for each contact removed from a chat. """


class Global:
    """ global hook specifications using a per-process singleton
    plugin manager instance.

    """
    _plugin_manager = None

    @classmethod
    def _get_plugin_manager(cls):
        if cls._plugin_manager is None:
            cls._plugin_manager = pm = pluggy.PluginManager(global_spec_name)
            pm.add_hookspecs(cls)
        return cls._plugin_manager

    @global_hookspec
    def dc_account_init(self, account):
        """ called when `Account::__init__()` function starts executing. """

    @global_hookspec
    def dc_account_after_shutdown(self, account, dc_context):
        """ Called after the account has been shutdown. """
