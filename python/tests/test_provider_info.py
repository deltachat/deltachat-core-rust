import pytest

from deltachat import const
from deltachat import provider


def test_provider_info_from_email():
    example = provider.Provider.from_email("email@example.com")
    assert example.overview_page == "https://providers.delta.chat/example.com"
    assert example.name == "Example"
    assert example.markdown == "\n..."
    assert example.status_date == "2018-09"
    assert example.status == const.DC_PROVIDER_STATUS_PREPARATION


def test_provider_info_from_domain():
    example = provider.Provider("example.com")
    assert example.overview_page == "https://providers.delta.chat/example.com"
    assert example.name == "Example"
    assert example.markdown == "\n..."
    assert example.status_date == "2018-09"
    assert example.status == const.DC_PROVIDER_STATUS_PREPARATION


def test_provider_info_none():
    with pytest.raises(provider.ProviderNotFoundError):
        provider.Provider.from_email("email@unexistent.no")
