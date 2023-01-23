"""Hooks for Python bindings to Delta Chat Core Rust CFFI."""

import pluggy

account_spec_name = "deltachat-account"
account_hookspec = pluggy.HookspecMarker(account_spec_name)
account_hookimpl = pluggy.HookimplMarker(account_spec_name)

global_spec_name = "deltachat-global"
global_hookspec = pluggy.HookspecMarker(global_spec_name)
global_hookimpl = pluggy.HookimplMarker(global_spec_name)


class PerAccount:
    """per-Account-instance hook specifications.

    All hooks are executed in a dedicated Event thread.
    Hooks are generally not allowed to block/last long as this
    blocks overall event processing on the python side.
    """

    @classmethod
    def _make_plugin_manager(cls):
        pm = pluggy.PluginManager(account_spec_name)
        pm.add_hookspecs(cls)
        return pm

    @account_hookspec
    def ac_process_ffi_event(self, ffi_event):
        """process a CFFI low level events for a given account.

        ffi_event has "name", "data1", "data2" values as specified
        with `DC_EVENT_* <https://c.delta.chat/group__DC__EVENT.html>`_.
        """

    @account_hookspec
    def ac_log_line(self, message):
        """log a message related to the account."""

    @account_hookspec
    def ac_configure_completed(self, success, comment):
        """Called after a configure process completed."""

    @account_hookspec
    def ac_incoming_message(self, message):
        """Called on any incoming message (both existing chats and contact requests)."""

    @account_hookspec
    def ac_outgoing_message(self, message):
        """Called on each outgoing message (both system and "normal")."""

    @account_hookspec
    def ac_reactions_changed(self, message):
        """Called when message reactions changed."""

    @account_hookspec
    def ac_message_delivered(self, message):
        """Called when an outgoing message has been delivered to SMTP.

        :param message: Message that was just delivered.
        """

    @account_hookspec
    def ac_chat_modified(self, chat):
        """Chat was created or modified regarding membership, avatar, title.

        :param chat: Chat which was modified.
        """

    @account_hookspec
    def ac_member_added(self, chat, contact, actor, message):
        """Called for each contact added to an accepted chat.

        :param chat: Chat where contact was added.
        :param contact: Contact that was added.
        :param actor: Who added the contact (None if it was our self-addr)
        :param message: The original system message that reports the addition.
        """

    @account_hookspec
    def ac_member_removed(self, chat, contact, actor, message):
        """Called for each contact removed from a chat.

        :param chat: Chat where contact was removed.
        :param contact: Contact that was removed.
        :param actor: Who removed the contact (None if it was our self-addr)
        :param message: The original system message that reports the removal.
        """


class Global:
    """global hook specifications using a per-process singleton
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
        """called when `Account::__init__()` function starts executing."""

    @global_hookspec
    def dc_account_extra_configure(self, account):
        """Called when account configuration successfully finished.

        This hook can be used to perform extra work before
        ac_configure_completed is called.
        """

    @global_hookspec
    def dc_account_after_shutdown(self, account):
        """Called after the account has been shutdown."""
