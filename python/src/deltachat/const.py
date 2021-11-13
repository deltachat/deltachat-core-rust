from typing import Any, List

from .capi import lib


def __getattr__(name: str) -> Any:
    if name.startswith("DC_"):
        return getattr(lib, name)
    return globals()[name]


def __dir__() -> List[str]:
    return sorted(name for name in dir(lib) if name.startswith("DC_"))
