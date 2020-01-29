from __future__ import print_function
import threading
from deltachat import capi, cutil, const, set_context_callback, clear_context_callback
from deltachat.capi import ffi
from deltachat.capi import lib
from deltachat.account import EventLogger


class EventThread(threading.Thread):
    def __init__(self, dc_context):
        self.dc_context = dc_context
        super(EventThread, self).__init__()
        self.setDaemon(1)

    def run(self):
        lib.dc_context_run(self.dc_context, lib.py_dc_callback)

    def stop(self):
        lib.dc_context_shutdown(self.dc_context)


if __name__ == "__main__":
    print("1")
    ctx = capi.lib.dc_context_new(ffi.NULL, ffi.NULL)
    print("2")
    ev_thread = EventThread(ctx)
    print("3 -- starting event thread")
    ev_thread.start()
    print("4 -- stopping event thread")
    ev_thread.stop()
