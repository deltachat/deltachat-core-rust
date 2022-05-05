"""
run with:

     pytest -vv --durations=10 bench_empty.py

to see timings of test setups.
"""

import pytest

BENCH_NUM = 3


class TestEmpty:
    def test_prepare_setup_measurings(self, acfactory):
        acfactory.get_online_accounts(BENCH_NUM)

    @pytest.mark.parametrize("num", range(0, BENCH_NUM + 1))
    def test_setup_online_accounts(self, acfactory, num):
        acfactory.get_online_accounts(num)
