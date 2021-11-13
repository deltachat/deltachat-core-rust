import sys

from . import capi, const, hookspec # noqa
from .capi import ffi  # noqa
from .account import Account  # noqa
from .message import Message  # noqa
from .contact import Contact  # noqa
from .chat import Chat        # noqa
from .hookspec import account_hookimpl, global_hookimpl  # noqa
from . import events

from pkg_resources import get_distribution, DistributionNotFound
try:
    __version__ = get_distribution(__name__).version
except DistributionNotFound:
    # package is not installed
    __version__ = "0.0.0.dev0-unknown"


def get_dc_event_name(integer, _DC_EVENTNAME_MAP={}):
    if not _DC_EVENTNAME_MAP:
        for name in dir(const):
            if name.startswith("DC_EVENT_"):
                _DC_EVENTNAME_MAP[getattr(const, name)] = name
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


register_global_plugin(events)


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
        ac.set_config("displayname", "bot")
        log = events.FFIEventLogger(ac)
        ac.add_account_plugin(log)

    for plugin in account_plugins or []:
        print("adding plugin", plugin)
        ac.add_account_plugin(plugin)

    if not ac.is_configured():
        assert args.email and args.password, (
            "you must specify --email and --password once to configure this database/account"
        )
        ac.set_config("addr", args.email)
        ac.set_config("mail_pw", args.password)
        ac.set_config("mvbox_move", "0")
        ac.set_config("mvbox_watch", "0")
        ac.set_config("sentbox_watch", "0")
        ac.set_config("bot", "1")
        configtracker = ac.configure()
        configtracker.wait_finish()

    # start IO threads and configure if neccessary
    ac.start_io()

    print("{}: waiting for message".format(ac.get_config("addr")))

    ac.wait_shutdown()
