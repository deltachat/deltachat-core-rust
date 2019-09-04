"""Provider info class."""

from .capi import ffi, lib
from .cutil import as_dc_charpointer, from_dc_charpointer


class ProviderNotFoundError(Exception):
    """The provider information was not found."""


class Provider(object):
    """Provider information.

    :param domain: The domain to get the provider info for, this is
    normally the part following the `@` of the domain.
    """

    def __init__(self, domain):
        provider = ffi.gc(
            lib.dc_provider_new_from_domain(as_dc_charpointer(domain)),
            lib.dc_provider_unref,
        )
        if provider == ffi.NULL:
            raise ProviderNotFoundError("Provider not found")
        self._provider = provider

    @classmethod
    def from_email(cls, email):
        """Create provider info from an email address.

        :param email: Email address to get provider info for.
        """
        return cls(email.split('@')[-1])

    @property
    def overview_page(self):
        """URL to the overview page of the provider on providers.delta.chat."""
        return from_dc_charpointer(
            lib.dc_provider_get_overview_page(self._provider))

    @property
    def name(self):
        """The name of the provider."""
        return from_dc_charpointer(lib.dc_provider_get_name(self._provider))

    @property
    def markdown(self):
        """Content of the information page, formatted as markdown."""
        return from_dc_charpointer(
            lib.dc_provider_get_markdown(self._provider))

    @property
    def status_date(self):
        """The date the provider info was last updated, as a string."""
        return from_dc_charpointer(
            lib.dc_provider_get_status_date(self._provider))

    @property
    def status(self):
        """The status of the provider information.

        This is one of the
        :attr:`deltachat.const.DC_PROVIDER_STATUS_OK`,
        :attr:`deltachat.const.DC_PROVIDER_STATUS_PREPARATION` or
        :attr:`deltachat.const.DC_PROVIDER_STATUS_BROKEN` constants.
        """
        return lib.dc_provider_get_status(self._provider)
