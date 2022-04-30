"""
run with:

     pytest -vv --durations=10 bench_empty.py

to see timings of test setups.
"""

import pytest


class TestEmpty:
    def test_prepare_setup_measurings(self, acfactory):
        acfactory.get_many_online_accounts(5)

    @pytest.mark.parametrize("num", range(0, 5))
    def test_setup_online_accounts(self, acfactory, num):
        acfactory.get_many_online_accounts(num)
