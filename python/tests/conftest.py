from __future__ import print_function

import os
import pytest
import py


from deltachat.testplugin import *  # noqa


@pytest.fixture(scope="session")
def data():
    class Data:
        def __init__(self):
            self.path = os.path.join(os.path.dirname(__file__), "data")

        def get_path(self, bn):
            fn = os.path.join(self.path, bn)
            assert os.path.exists(fn)
            return fn
    return Data()


@pytest.fixture(scope='session')
def datadir():
    """The py.path.local object of the test-data/ directory."""
    for path in reversed(py.path.local(__file__).parts()):
        datadir = path.join('test-data')
        if datadir.isdir():
            return datadir
    else:
        pytest.skip('test-data directory not found')


def wait_configuration_progress(account, min_target, max_target=1001):
    min_target = min(min_target, max_target)
    while 1:
        event = account._evtracker.get_matching("DC_EVENT_CONFIGURE_PROGRESS")
        if event.data1 >= min_target and event.data1 <= max_target:
            print("** CONFIG PROGRESS {}".format(min_target), account)
            break


def wait_securejoin_inviter_progress(account, target):
    while 1:
        event = account._evtracker.get_matching("DC_EVENT_SECUREJOIN_INVITER_PROGRESS")
        if event.data2 >= target:
            print("** SECUREJOINT-INVITER PROGRESS {}".format(target), account)
            break
