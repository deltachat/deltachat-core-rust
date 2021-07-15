import pytest


@pytest.fixture
def acprovider(acfactory, real_provider_config):
    ac = acfactory.make_account_from_real_config(real_provider_config)
    ac.update_config(real_provider_config)
    ac._configtracker = ac.configure()
    return ac


@pytest.fixture
def actest(acfactory):
    return acfactory.get_online_configuring_account()


def test_configure_success(acfactory, acprovider, lp):
    lp.sec("waiting for successful configuration of provider account")
    acfactory.wait_configure_and_start_io()

    assert acprovider.is_configured()
    for name in ("inbox", "mvbox", "sentbox"):
        folder = acprovider.get_config("configured_" + name + "_folder")
        if not folder:
            lp.sec("found no {} folder".format(name))
            continue

        lp.sec("removing provider account IMAP folder {}".format(folder))
        acprovider.direct_imap.select_folder(folder)
        acprovider.direct_imap.delete("1:*")


def test_basic_send_receive(acprovider, actest, acfactory, lp):
    acfactory.wait_configure_and_start_io()

    lp.sec("sending message from test account to provider account")
    chat = actest.create_chat(acprovider)
    chat.send_text("hello")

    lp.sec("receiving message with the provider account")
    msg = acprovider._evtracker.wait_next_messages_changed()
    assert msg.chat.is_deaddrop() and not msg.is_encrypted()

    lp.sec("sending message back from provider to test account")
    back_chat = acprovider.create_chat(actest)
    back_chat.send_text("world")

    lp.sec("waiting with test account for provider mail")
    msg = actest._evtracker.wait_next_incoming_message()
    assert msg.text == "world"
    assert msg.is_encrypted()


def test_group_messages(acprovider, actest, acfactory, lp):
    acfactory.wait_configure_and_start_io()

    lp.sec("sending message from test account to provider account")
    chat = actest.create_chat(acprovider)
    chat.send_text("hello")

    lp.sec("receiving message with the provider account")
    msg = acprovider._evtracker.wait_next_messages_changed()
    assert msg.chat.is_deaddrop() and not msg.is_encrypted()

    lp.sec("sending message back from provider to test account")
    back_chat = acprovider.create_chat(actest)
    back_chat.send_text("world")

    lp.sec("waiting with test account for provider mail")
    msg = actest._evtracker.wait_next_incoming_message()
    assert msg.text == "world"
    assert msg.is_encrypted()
