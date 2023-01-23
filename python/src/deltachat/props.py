"""Helpers for properties."""


def with_doc(f):
    return property(f, None, None, f.__doc__)


# copied over unmodified from
# https://github.com/devpi/devpi/blob/master/common/devpi_common/types.py
def cached(f):
    """returns a cached property that is calculated by function f."""

    def get(self):
        try:
            return self._property_cache[f]
        except AttributeError:
            self._property_cache = {}
        except KeyError:
            pass
        res = f(self)
        self._property_cache[f] = res
        return res

    def set(self, val):
        propcache = self.__dict__.setdefault("_property_cache", {})
        propcache[f] = val

    def fdel(self):
        propcache = self.__dict__.setdefault("_property_cache", {})
        del propcache[f]

    return property(get, set, fdel)
