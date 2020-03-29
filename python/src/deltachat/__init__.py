import sys

from . import capi, const, hookspec
from .capi import ffi
from .account import Account  # noqa
from .message import Message  # noqa
from .contact import Contact  # noqa
from .chat import Chat        # noqa
from .hookspec import account_hookimpl, global_hookimpl  # noqa
from . import eventlogger

from pkg_resources import get_distribution, DistributionNotFound
try:
    __version__ = get_distribution(__name__).version
except DistributionNotFound:
    # package is not installed
    __version__ = "0.0.0.dev0-unknown"


_DC_CALLBACK_MAP = {}


@capi.ffi.def_extern()
def py_dc_callback(ctx, evt, data1, data2):
    """The global event handler.

    CFFI only allows us to set one global event handler, so this one
    looks up the correct event handler for the given context.
    """
    try:
        callback = _DC_CALLBACK_MAP.get(ctx, lambda *a: 0)
    except AttributeError:
        # we are in a deep in GC-free/interpreter shutdown land
        # nothing much better to do here than:
        return 0

    # the following code relates to the deltachat/_build.py's helper
    # function which provides us signature info of an event call
    evt_name = get_dc_event_name(evt)
    event_sig_types = capi.lib.dc_get_event_signature_types(evt)
    if data1 and event_sig_types & 1:
        data1 = ffi.string(ffi.cast('char*', data1)).decode("utf8")
    if data2 and event_sig_types & 2:
        data2 = ffi.string(ffi.cast('char*', data2)).decode("utf8")
        try:
            if isinstance(data2, bytes):
                data2 = data2.decode("utf8")
        except UnicodeDecodeError:
            # XXX ignoring the decode error is not quite correct but for now
            # i don't want to hunt down encoding problems in the c lib
            pass
    try:
        ret = callback(ctx, evt_name, data1, data2)
        if ret is None:
            ret = 0
        assert isinstance(ret, int), repr(ret)
        if event_sig_types & 4:
            return ffi.cast('uintptr_t', ret)
        elif event_sig_types & 8:
            return ffi.cast('int', ret)
    except:  # noqa
        raise
        ret = 0
    return ret


def set_context_callback(dc_context, func):
    _DC_CALLBACK_MAP[dc_context] = func


def clear_context_callback(dc_context):
    try:
        _DC_CALLBACK_MAP.pop(dc_context, None)
    except AttributeError:
        pass


def get_dc_event_name(integer, _DC_EVENTNAME_MAP={}):
    if not _DC_EVENTNAME_MAP:
        for name, val in vars(const).items():
            if name.startswith("DC_EVENT_"):
                _DC_EVENTNAME_MAP[val] = name
    return _DC_EVENTNAME_MAP[integer]


def register_global_plugin(plugin):
    """ Register a global plugin which implements one or more
    of the :class:`deltachat.hookspec.Global` hooks.
    """
    gm = hookspec.Global._get_plugin_manager()
    gm.register(plugin)
    gm.check_pending()


def unregister_global_plugin(plugin):
    gm = hookspec.Global._get_plugin_manager()
    gm.unregister(plugin)


register_global_plugin(eventlogger)


def run_cmdline(argv=None, account_plugins=None):
    """ Run a simple default command line app, registering the specified
    account plugins. """
    import argparse
    if argv is None:
        argv = sys.argv

    parser = argparse.ArgumentParser(prog=argv[0] if argv else None)
    parser.add_argument("db", action="store", help="database file")
    parser.add_argument("--show-ffi", action="store_true", help="show low level ffi events")
    parser.add_argument("--email", action="store", help="email address")
    parser.add_argument("--password", action="store", help="password")

    args = parser.parse_args(argv[1:])

    ac = Account(args.db)

    if args.show_ffi:
        log = eventlogger.FFIEventLogger(ac, "bot")
        ac.add_account_plugin(log)

    if not ac.is_configured():
        assert args.email and args.password, (
            "you must specify --email and --password once to configure this database/account"
        )
        ac.set_config("addr", args.email)
        ac.set_config("mail_pw", args.password)
        ac.set_config("mvbox_move", "0")
        ac.set_config("mvbox_watch", "0")
        ac.set_config("sentbox_watch", "0")

    for plugin in account_plugins or []:
        ac.add_account_plugin(plugin)

    # start IO threads and configure if neccessary
    ac.start()

    print("{}: waiting for message".format(ac.get_config("addr")))

    ac.wait_shutdown()
