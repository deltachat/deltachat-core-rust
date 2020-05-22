from __future__ import print_function
from deltachat import capi
from deltachat.capi import ffi, lib

if __name__ == "__main__":
    ctx = capi.lib.dc_context_new(ffi.NULL, ffi.NULL)
    lib.dc_stop_io(ctx)
