#!/usr/bin/env python3
"""
Testing webxdc iroh connectivity

If you want to debug iroh at rust-trace/log level set

    RUST_LOG=iroh_net=trace,iroh_gossip=trace
"""

import pytest

import time
import os
import sys
import logging
import random
import itertools
import sys

from deltachat_rpc_client import DeltaChat, EventType, SpecialContactId


@pytest.fixture()
def path_to_webxdc():
    return "../test-data/webxdc/chess.xdc"


def test_realtime_sequentially(acfactory, path_to_webxdc):
    """Test two peers trying to establish connection sequentially."""
    ac1, ac2 = acfactory.get_online_accounts(2)
    ac1.create_chat(ac2)
    ac2.create_chat(ac1)
    acfactory.send_message(from_account=ac1, to_account=ac2, text="ping0")
    snapshot = ac2.get_message_by_id(ac2.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "ping0"

    def log(msg):
        print()
        print("*" * 80 + "\n" + msg + "\n", file=sys.stderr)
        print()

    # share a webxdc app between ac1 and ac2
    ac1_webxdc_msg = acfactory.send_message(from_account=ac1, to_account=ac2, text="play", file=path_to_webxdc)
    ac2_webxdc_msg = ac2.get_message_by_id(ac2.wait_for_incoming_msg_event().msg_id)
    snapshot = ac2_webxdc_msg.get_snapshot()
    assert snapshot.text == "play"

    # send iroh announcements sequentially
    log("sending ac1 -> ac2 realtime advertisement and additional message")
    ac1._rpc.send_webxdc_realtime_advertisement(ac1.id, ac1_webxdc_msg.id)
    acfactory.send_message(from_account=ac1, to_account=ac2, text="ping1")

    log("waiting for incoming message on ac2")
    snapshot = ac2.get_message_by_id(ac2.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "ping1"

    log("sending ac2 -> ac1 realtime advertisement and additional message")
    ac2._rpc.send_webxdc_realtime_advertisement(ac2.id, ac2_webxdc_msg.id)
    acfactory.send_message(from_account=ac2, to_account=ac1, text="ping2")

    log("waiting for incoming message on ac1")
    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "ping2"

    log("sending realtime data ac1 -> ac2")
    ac1._rpc.send_webxdc_realtime_data(ac1.id, ac1_webxdc_msg.id, [13, 15, 17])

    log("ac2: waiting for realtime data")
    while 1:
        event = ac2.wait_for_event()
        if event.kind == EventType.WEBXDC_REALTIME_DATA:
            assert event.data == [13, 15, 17]
            break


def test_realtime_simultaneously(acfactory, path_to_webxdc):
    """Test two peers trying to establish connection simultaneously."""
    ac1, ac2 = acfactory.get_online_accounts(2)
    ac1.create_chat(ac2)
    ac2.create_chat(ac1)
    acfactory.send_message(from_account=ac1, to_account=ac2, text="ping0")
    snapshot = ac2.get_message_by_id(ac2.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "ping0"

    def log(msg):
        print()
        print("*" * 80 + "\n" + msg + "\n", file=sys.stderr)
        print()

    # share a webxdc app between ac1 and ac2
    ac1_webxdc_msg = acfactory.send_message(from_account=ac1, to_account=ac2, text="play", file=path_to_webxdc)
    ac2_webxdc_msg = ac2.get_message_by_id(ac2.wait_for_incoming_msg_event().msg_id)
    snapshot = ac2_webxdc_msg.get_snapshot()
    assert snapshot.text == "play"

    # send iroh announcements simultaneously
    log("sending ac1 -> ac2 realtime advertisement and additional message")
    ac1._rpc.send_webxdc_realtime_advertisement(ac1.id, ac1_webxdc_msg.id)
    acfactory.send_message(from_account=ac1, to_account=ac2, text="ping1")

    log("sending ac2 -> ac1 realtime advertisement and additional message")
    ac2._rpc.send_webxdc_realtime_advertisement(ac2.id, ac2_webxdc_msg.id)
    acfactory.send_message(from_account=ac2, to_account=ac1, text="ping2")

    # Ensure that advertisements have been received.

    log("waiting for incoming message on ac2")
    snapshot = ac2.get_message_by_id(ac2.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "ping1"

    log("waiting for incoming message on ac1")
    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "ping2"

    log("sending realtime data ac1 -> ac2")
    ac1._rpc.send_webxdc_realtime_data(ac1.id, ac1_webxdc_msg.id, [13, 15, 17])

    log("ac2: waiting for realtime data")
    while 1:
        event = ac2.wait_for_event()
        if event.kind == EventType.WEBXDC_REALTIME_DATA:
            assert event.data == [13, 15, 17]
            break
