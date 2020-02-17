from __future__ import print_function
import os
import py
import pytest
import requests
import time
from deltachat import Account
from deltachat import const
from deltachat.capi import lib
import tempfile


def pytest_addoption(parser):
    parser.addoption(
        "--liveconfig", action="store", default=None,
        help="a file with >=2 lines where each line "
             "contains NAME=VALUE config settings for one account"
    )
    parser.addoption(
        "--ignored", action="store_true",
        help="Also run tests marked with the ignored marker",
    )


def pytest_configure(config):
    config.addinivalue_line(
        "markers", "ignored: Mark test as bing slow, skipped unless --ignored is used."
    )
    cfg = config.getoption('--liveconfig')
    if not cfg:
        cfg = os.getenv('DCC_NEW_TMP_EMAIL')
        if cfg:
            config.option.liveconfig = cfg


def pytest_runtest_setup(item):
    if (list(item.iter_markers(name="ignored"))
            and not item.config.getoption("ignored")):
        pytest.skip("Ignored tests not requested, use --ignored")


def pytest_report_header(config, startdir):
    summary = []

    t = tempfile.mktemp()
    try:
        ac = Account(t, eventlogging=False)
        info = ac.get_info()
        ac.shutdown()
    finally:
        os.remove(t)
    summary.extend(['Deltachat core={} sqlite={}'.format(
         info['deltachat_core_version'],
         info['sqlite_version'],
     )])

    cfg = config.option.liveconfig
    if cfg:
        if "#" in cfg:
            url, token = cfg.split("#", 1)
            summary.append('Liveconfig provider: {}#<token ommitted>'.format(url))
        else:
            summary.append('Liveconfig file: {}'.format(cfg))
    return summary


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


class SessionLiveConfigFromFile:
    def __init__(self, fn):
        self.fn = fn
        self.configlist = []
        for line in open(fn):
            if line.strip() and not line.strip().startswith('#'):
                d = {}
                for part in line.split():
                    name, value = part.split("=")
                    d[name] = value
                self.configlist.append(d)

    def get(self, index):
        return self.configlist[index]

    def exists(self):
        return bool(self.configlist)


class SessionLiveConfigFromURL:
    def __init__(self, url):
        self.configlist = []
        self.url = url

    def get(self, index):
        try:
            return self.configlist[index]
        except IndexError:
            assert index == len(self.configlist), index
            res = requests.post(self.url)
            if res.status_code != 200:
                pytest.skip("creating newtmpuser failed {!r}".format(res))
            d = res.json()
            config = dict(addr=d["email"], mail_pw=d["password"])
            self.configlist.append(config)
            return config

    def exists(self):
        return bool(self.configlist)


@pytest.fixture(scope="session")
def session_liveconfig(request):
    liveconfig_opt = request.config.option.liveconfig
    if liveconfig_opt:
        if liveconfig_opt.startswith("http"):
            return SessionLiveConfigFromURL(liveconfig_opt)
        else:
            return SessionLiveConfigFromFile(liveconfig_opt)


@pytest.fixture(scope='session')
def datadir():
    """The py.path.local object of the test-data/ directory."""
    for path in reversed(py.path.local(__file__).parts()):
        datadir = path.join('test-data')
        if datadir.isdir():
            return datadir
    else:
        pytest.skip('test-data directory not found')


@pytest.fixture
def acfactory(pytestconfig, tmpdir, request, session_liveconfig, datadir):

    class AccountMaker:
        def __init__(self):
            self.live_count = 0
            self.offline_count = 0
            self._finalizers = []
            self.init_time = time.time()
            self._generated_keys = ["alice", "bob", "charlie",
                                    "dom", "elena", "fiona"]

        def finalize(self):
            while self._finalizers:
                fin = self._finalizers.pop()
                fin()

        def make_account(self, path, logid):
            ac = Account(path, logid=logid)
            self._finalizers.append(ac.shutdown)
            return ac

        def get_unconfigured_account(self):
            self.offline_count += 1
            tmpdb = tmpdir.join("offlinedb%d" % self.offline_count)
            ac = self.make_account(tmpdb.strpath, logid="ac{}".format(self.offline_count))
            ac._evlogger.init_time = self.init_time
            ac._evlogger.set_timeout(2)
            return ac

        def _preconfigure_key(self, account, addr):
            # Only set a key if we haven't used it yet for another account.
            if self._generated_keys:
                keyname = self._generated_keys.pop(0)
                fname_pub = "key/{name}-public.asc".format(name=keyname)
                fname_sec = "key/{name}-secret.asc".format(name=keyname)
                account._preconfigure_keypair(addr,
                                              datadir.join(fname_pub).read(),
                                              datadir.join(fname_sec).read())

        def get_configured_offline_account(self):
            ac = self.get_unconfigured_account()

            # do a pseudo-configured account
            addr = "addr{}@offline.org".format(self.offline_count)
            ac.set_config("addr", addr)
            self._preconfigure_key(ac, addr)
            lib.dc_set_config(ac._dc_context, b"configured_addr", addr.encode("ascii"))
            ac.set_config("mail_pw", "123")
            lib.dc_set_config(ac._dc_context, b"configured_mail_pw", b"123")
            lib.dc_set_config(ac._dc_context, b"configured", b"1")
            return ac

        def get_online_config(self, pre_generated_key=True):
            if not session_liveconfig:
                pytest.skip("specify DCC_NEW_TMP_EMAIL or --liveconfig")
            configdict = session_liveconfig.get(self.live_count)
            self.live_count += 1
            if "e2ee_enabled" not in configdict:
                configdict["e2ee_enabled"] = "1"

            # Enable strict certificate checks for online accounts
            configdict["imap_certificate_checks"] = str(const.DC_CERTCK_STRICT)
            configdict["smtp_certificate_checks"] = str(const.DC_CERTCK_STRICT)

            tmpdb = tmpdir.join("livedb%d" % self.live_count)
            ac = self.make_account(tmpdb.strpath, logid="ac{}".format(self.live_count))
            if pre_generated_key:
                self._preconfigure_key(ac, configdict['addr'])
            ac._evlogger.init_time = self.init_time
            ac._evlogger.set_timeout(30)
            return ac, dict(configdict)

        def get_online_configuring_account(self, mvbox=False, sentbox=False,
                                           pre_generated_key=True, config={}):
            ac, configdict = self.get_online_config(
                pre_generated_key=pre_generated_key)
            configdict.update(config)
            ac.configure(**configdict)
            ac.start_threads(mvbox=mvbox, sentbox=sentbox)
            return ac

        def get_one_online_account(self, pre_generated_key=True):
            ac1 = self.get_online_configuring_account(
                pre_generated_key=pre_generated_key)
            wait_successful_IMAP_SMTP_connection(ac1)
            wait_configuration_progress(ac1, 1000)
            return ac1

        def get_two_online_accounts(self):
            ac1 = self.get_online_configuring_account()
            ac2 = self.get_online_configuring_account()
            wait_successful_IMAP_SMTP_connection(ac1)
            wait_configuration_progress(ac1, 1000)
            wait_successful_IMAP_SMTP_connection(ac2)
            wait_configuration_progress(ac2, 1000)
            return ac1, ac2

        def clone_online_account(self, account, pre_generated_key=True):
            self.live_count += 1
            tmpdb = tmpdir.join("livedb%d" % self.live_count)
            ac = self.make_account(tmpdb.strpath, logid="ac{}".format(self.live_count))
            if pre_generated_key:
                self._preconfigure_key(ac, account.get_config("addr"))
            ac._evlogger.init_time = self.init_time
            ac._evlogger.set_timeout(30)
            ac.configure(addr=account.get_config("addr"), mail_pw=account.get_config("mail_pw"))
            ac.start_threads()
            return ac

    am = AccountMaker()
    request.addfinalizer(am.finalize)
    return am


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


def wait_configuration_progress(account, min_target, max_target=1001):
    min_target = min(min_target, max_target)
    while 1:
        evt_name, data1, data2 = \
            account._evlogger.get_matching("DC_EVENT_CONFIGURE_PROGRESS")
        if data1 >= min_target and data1 <= max_target:
            print("** CONFIG PROGRESS {}".format(min_target), account)
            break


def wait_securejoin_inviter_progress(account, target):
    while 1:
        evt_name, data1, data2 = \
            account._evlogger.get_matching("DC_EVENT_SECUREJOIN_INVITER_PROGRESS")
        if data2 >= target:
            print("** SECUREJOINT-INVITER PROGRESS {}".format(target), account)
            break


def wait_successful_IMAP_SMTP_connection(account):
    imap_ok = smtp_ok = False
    while not imap_ok or not smtp_ok:
        evt_name, data1, data2 = \
            account._evlogger.get_matching("DC_EVENT_(IMAP|SMTP)_CONNECTED")
        if evt_name == "DC_EVENT_IMAP_CONNECTED":
            imap_ok = True
            print("** IMAP OK", account)
        if evt_name == "DC_EVENT_SMTP_CONNECTED":
            smtp_ok = True
            print("** SMTP OK", account)
    print("** IMAP and SMTP logins successful", account)


def wait_msgs_changed(account, chat_id, msg_id=None):
    ev = account._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
    assert ev[1] == chat_id
    if msg_id is not None:
        assert ev[2] == msg_id
    return ev[2]
