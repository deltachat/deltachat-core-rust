from .capi import lib
from .capi import ffi


def as_dc_charpointer(obj):
    if obj == ffi.NULL or obj is None:
        return ffi.NULL
    if not isinstance(obj, bytes):
        return obj.encode("utf8")
    return obj


def iter_array(dc_array_t, constructor):
    for i in range(0, lib.dc_array_get_cnt(dc_array_t)):
        yield constructor(lib.dc_array_get_id(dc_array_t, i))


def from_dc_charpointer(obj):
    return ffi.string(obj).decode("utf8")
