from __future__ import print_function


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
