from .capi import lib


for name in dir(lib):
    if name.startswith("DC_"):
        globals()[name] = getattr(lib, name)
del name

DC_IMEX_EXPORT_SELF_KEYS = 1
DC_IMEX_IMPORT_SELF_KEYS = 2
DC_IMEX_EXPORT_BACKUP = 11
DC_IMEX_IMPORT_BACKUP = 12