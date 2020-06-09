from .capi import lib


for name in dir(lib):
    if name.startswith("DC_"):
        globals()[name] = getattr(lib, name)
del name
