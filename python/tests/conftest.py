from __future__ import print_function
import os
import pytest
import time
from deltachat import Account
from deltachat import props
from deltachat.capi import lib
import tempfile


def pytest_addoption(parser):
    parser.addoption(
        "--liveconfig", action="store", default=None,
        help="a file with >=2 lines where each line "
             "contains NAME=VALUE config settings for one account"
    )


def pytest_report_header(config, startdir):
    t = tempfile.mktemp()
    try:
        ac = Account(t)
        info = ac.get_info()
        del ac
    finally:
        os.remove(t)
    return "Deltachat core={} rpgp={} openssl={} sqlite={}".format(
        info["deltachat_core_version"],
        info["rpgp_enabled"],
        info['openssl_version'],
        info['sqlite_version'],
    )


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


@pytest.fixture
def acfactory(pytestconfig, tmpdir, request):
    fn = pytestconfig.getoption("--liveconfig")

    class AccountMaker:
        def __init__(self):
            self.live_count = 0
            self.offline_count = 0
            self._finalizers = []
            request.addfinalizer(self.finalize)
            self.init_time = time.time()

        def finalize(self):
            while self._finalizers:
                fin = self._finalizers.pop()
                fin()

        @props.cached
        def configlist(self):
            configlist = []
            for line in open(fn):
                if line.strip():
                    d = {}
                    for part in line.split():
                        name, value = part.split("=")
                        d[name] = value
                    configlist.append(d)
            return configlist

        def get_unconfigured_account(self):
            self.offline_count += 1
            tmpdb = tmpdir.join("offlinedb%d" % self.offline_count)
            ac = Account(tmpdb.strpath, logid="ac{}".format(self.offline_count))
            ac._evlogger.init_time = self.init_time
            ac._evlogger.set_timeout(2)
            return ac

        def get_configured_offline_account(self):
            ac = self.get_unconfigured_account()

            # do a pseudo-configured account
            addr = "addr{}@offline.org".format(self.offline_count)
            ac.set_config("addr", addr)
            lib.dc_set_config(ac._dc_context, b"configured_addr", addr.encode("ascii"))
            ac.set_config("mail_pw", "123")
            lib.dc_set_config(ac._dc_context, b"configured_mail_pw", b"123")
            lib.dc_set_config(ac._dc_context, b"configured", b"1")
            return ac

        def get_online_configuring_account(self):
            if not fn:
                pytest.skip("specify a --liveconfig file to run tests with real accounts")
            self.live_count += 1
            configdict = self.configlist.pop(0)
            tmpdb = tmpdir.join("livedb%d" % self.live_count)
            ac = Account(tmpdb.strpath, logid="ac{}".format(self.live_count))
            ac._evlogger.init_time = self.init_time
            ac._evlogger.set_timeout(30)
            ac.configure(**configdict)
            ac.start_threads()
            self._finalizers.append(ac.stop_threads)
            return ac

    return AccountMaker()


@pytest.fixture
def tmp_db_path(tmpdir):
    return tmpdir.join("test.db").strpath


@pytest.fixture
def lp():
    class Printer:
        def sec(self, msg):
            print()
            print("=" * 10, msg, "=" * 10)

        def step(self, msg):
            print("-" * 5, "step " + msg, "-" * 5)
    return Printer()


def wait_configuration_progress(account, target):
    while 1:
        evt_name, data1, data2 = \
            account._evlogger.get_matching("DC_EVENT_CONFIGURE_PROGRESS")
        if data1 >= target:
            print("** CONFIG PROGRESS {}".format(target), account)
            break


def wait_successful_IMAP_SMTP_connection(account):
    imap_ok = smtp_ok = False
    while not imap_ok or not smtp_ok:
        evt_name, data1, data2 = \
            account._evlogger.get_matching("DC_EVENT_(IMAP|SMTP)_CONNECTED")
        if evt_name == "DC_EVENT_IMAP_CONNECTED":
            imap_ok = True
        if evt_name == "DC_EVENT_SMTP_CONNECTED":
            smtp_ok = True
    print("** IMAP and SMTP logins successful", account)


def wait_msgs_changed(account, chat_id, msg_id=None):
    ev = account._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
    assert ev[1] == chat_id
    if msg_id is not None:
        assert ev[2] == msg_id
    return ev[2]
