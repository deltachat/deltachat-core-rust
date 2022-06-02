from datetime import datetime, timezone
from typing import Callable, Generator, Optional, TypeVar

from .capi import ffi, lib

T = TypeVar("T")


def as_dc_charpointer(obj):
    if obj == ffi.NULL or obj is None:
        return ffi.NULL
    if not isinstance(obj, bytes):
        return obj.encode("utf8")
    return obj


def iter_array(dc_array_t, constructor: Callable[[int], T]) -> Generator[T, None, None]:
    for i in range(0, lib.dc_array_get_cnt(dc_array_t)):
        yield constructor(lib.dc_array_get_id(dc_array_t, i))


def from_dc_charpointer(obj) -> str:
    if obj != ffi.NULL:
        return ffi.string(ffi.gc(obj, lib.dc_str_unref)).decode("utf8")
    raise ValueError


def from_optional_dc_charpointer(obj) -> Optional[str]:
    if obj != ffi.NULL:
        return ffi.string(ffi.gc(obj, lib.dc_str_unref)).decode("utf8")
    return None


class DCLot:
    def __init__(self, dc_lot) -> None:
        self._dc_lot = dc_lot

    def id(self) -> int:
        return lib.dc_lot_get_id(self._dc_lot)

    def state(self):
        return lib.dc_lot_get_state(self._dc_lot)

    def text1(self):
        return from_dc_charpointer(lib.dc_lot_get_text1(self._dc_lot))

    def text1_meaning(self):
        return lib.dc_lot_get_text1_meaning(self._dc_lot)

    def text2(self):
        return from_dc_charpointer(lib.dc_lot_get_text2(self._dc_lot))

    def timestamp(self):
        ts = lib.dc_lot_get_timestamp(self._dc_lot)
        if ts == 0:
            return None
        return datetime.fromtimestamp(ts, timezone.utc)
