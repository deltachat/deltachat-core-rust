from deltachat import capi, const
from deltachat.capi import ffi
from deltachat.account import Account  # noqa

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
