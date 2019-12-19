"""Provider info class."""

from .capi import ffi, lib
from .cutil import as_dc_charpointer, from_dc_charpointer
import json

class ProviderNotFoundError(Exception):
    """The provider information was not found."""


class Provider(object):
    """Provider information.

    :param domain: The domain to get the provider info for, this is
    normally the part following the `@` of the domain.
    """

    def __init__(self, domain):
        provider = from_dc_charpointer(
            lib.dc_provider_json_from_domain(as_dc_charpointer(domain))
        )
        if provider == "":
            raise ProviderNotFoundError("Provider not found")
        self._provider = json.loads(provider)

    @classmethod
    def from_email(cls, email):
        """Create provider info from an email address.

        :param email: Email address to get provider info for.
        """
        return cls(email.split('@')[-1])

    @property
    def overview_page(self):
        """URL to the overview page of the provider on providers.delta.chat."""
        return "https://providers.delta.chat/" + self._provider['overview_page']

    @property
    def name(self):
        """The name of the provider."""
        return self._provider['name']

    @property
    def markdown(self):
        """Content of the information page, formatted as markdown."""
        return self._provider['markdown']

    @property
    def status_date(self):
        """The date the provider info was last updated, as a string."""
        return self._provider['status']['date']

    @property
    def status(self):
        """The status of the provider information.

        This is 
        :attr:`"OK"`,
        :attr:`"PREPARATION"` or
        :attr:`"BROKEN"`.
        """
        return self._provider['status']['state']
