"""Provider info class."""

from .capi import ffi, lib
from .cutil import as_dc_charpointer, from_dc_charpointer


class ProviderNotFoundError(Exception):
    """The provider information was not found."""


class Provider(object):
    """
    Provider information.

    :param domain: The email to get the provider info for.
    """

    def __init__(self, account, addr) -> None:
        provider = ffi.gc(
            lib.dc_provider_new_from_email(account._dc_context, as_dc_charpointer(addr)),
            lib.dc_provider_unref,
        )
        if provider == ffi.NULL:
            raise ProviderNotFoundError("Provider not found")
        self._provider = provider

    @property
    def overview_page(self) -> str:
        """URL to the overview page of the provider on providers.delta.chat."""
        return from_dc_charpointer(lib.dc_provider_get_overview_page(self._provider))

    @property
    def get_before_login_hints(self) -> str:
        """Should be shown to the user on login."""
        return from_dc_charpointer(lib.dc_provider_get_before_login_hint(self._provider))

    @property
    def status(self) -> int:
        """The status of the provider information.

        This is one of the
        :attr:`deltachat.const.DC_PROVIDER_STATUS_OK`,
        :attr:`deltachat.const.DC_PROVIDER_STATUS_PREPARATION` or
        :attr:`deltachat.const.DC_PROVIDER_STATUS_BROKEN` constants.
        """
        return lib.dc_provider_get_status(self._provider)
