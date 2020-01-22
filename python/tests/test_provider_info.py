import pytest

from deltachat import provider


def test_provider_info_none():
    with pytest.raises(provider.ProviderNotFoundError):
        provider.Provider("email@unexistent.no")
