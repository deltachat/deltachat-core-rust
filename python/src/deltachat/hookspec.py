""" Hooks for python bindings """

import pluggy

name = "deltachat"

hookspec = pluggy.HookspecMarker(name)
hookimpl = pluggy.HookimplMarker(name)
_plugin_manager = None


def get_plugin_manager():
    global _plugin_manager
    if _plugin_manager is None:
        _plugin_manager = pluggy.PluginManager(name)
        _plugin_manager.add_hookspecs(DeltaChatHookSpecs)
    return _plugin_manager


class DeltaChatHookSpecs:
    """ Plugin Hook specifications for Python bindings to Delta Chat CFFI. """

    @hookspec
    def process_low_level_event(self, account, event_name, data1, data2):
        """ process a CFFI low level events for a given account. """
