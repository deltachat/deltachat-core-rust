import pytest


@pytest.fixture
def relay():
    return Relay()


class Relay:
    def __init__(self):
        self.peers = {}

    def make_peers(self, num):
        for i in range(num):
            newpeer = Peer(relay=self, num=i)
            self.peers[newpeer.id] = newpeer
        return self.peers.values()

    def receive_all(self, peers=None):
        peers = peers if peers else self.peers.values()
        for peer in peers:
            # drain peer mailbox by reading messages from each sender separately
            for from_peer in self.peers.values():
                for msg in peer.from2mailbox.pop(from_peer, []):
                    msg.receive_imf(peer)
                    peer.current_clock = max(peer.current_clock, msg.clock) + 1

    def assert_same_members(self):
        peers = list(self.peers.values())
        for peer1, peer2 in zip(peers, peers[1:]):
            assert peer1.members == peer2.members
            nums = ",".join(peer1.members)
            print(f"{peer1.id} and {peer2.id} have same members {nums}")


class Message:
    def __init__(self, sender, **payload):
        self.sender = sender
        self.recipients = list(sender.members)
        self.relay = sender.relay
        self.payload = payload
        self.clock = sender.current_clock

    def __repr__(self):
        nums = ",".join(self.recipients)
        return f"<{self.__class__.__name__} {self.sender.id}->{nums} {self.payload}"

    def send(self):
        for peer_id in self.sender.members:
            peer = self.relay.peers[peer_id]
            peer.from2mailbox.setdefault(self.sender, []).append(self)


class AddMemberMessage(Message):
    def receive_imf(self, peer):
        peer.members.add(self.payload["newmember"])
        peer.members.update(self.recipients)


class DelMemberMessage(Message):
    def receive_imf(self, peer):
        member = self.payload["member"]
        if member in peer.members:
            peer.members.remove(member)


class ChatMessage(Message):
    def receive_imf(self, peer):
        if peer.current_clock < self.clock:
            peer.members = set(self.recipients)
            peer.current_clock = self.clock


class Peer:
    """A peer in a group"""

    def __init__(self, relay, num):
        self.relay = relay
        self.id = f"p{num}"
        self.members = set()
        self.from2mailbox = {}
        self.current_clock = 0

    def __eq__(self, other):
        return self.id == other.id

    def __hash__(self):
        return int(self.id[1:])

    def __repr__(self):
        clock = self.current_clock
        return f"<Peer {self.id} members={','.join(self.members)} clock={clock}>"

    def immediate_create_group(self, peers):
        assert not self.members
        self.members.add(self.id)
        for peer in peers:
            self.send_add_member(peer.id)
        self.relay.receive_all()

    def send_add_member(self, newmember):
        assert isinstance(newmember, str)
        self.members.add(newmember)
        AddMemberMessage(self, newmember=newmember).send()

    def send_del_member(self, member):
        assert isinstance(member, str)
        assert member in self.members
        msg = DelMemberMessage(self, member=member)
        msg.send()
        self.members.remove(member)

    def send_chatmessage(self):
        ChatMessage(self).send()


### Tests


def test_add_and_remove(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    # create group
    p0.immediate_create_group([p1])
    assert p0.members == p1.members == set([p0.id, p1.id])

    # add members
    p0.send_add_member(p2.id)
    p0.send_add_member(p3.id)
    relay.receive_all()
    relay.assert_same_members()

    p3.send_del_member(p0.id)
    relay.receive_all()
    relay.assert_same_members()


def test_concurrent_add(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    p0.immediate_create_group([p1])
    # concurrent adding and then let base set send a chat message
    p1.send_add_member(p2.id)
    p0.send_add_member(p3.id)

    # now p0 and p1 send a regular message
    p0.send_chatmessage()
    p1.send_chatmessage()
    relay.receive_all()
    # only now do p0 and p1 know of each others additions
    # so p0 or p1 needs to send another message to get consistent membership
    p0.send_chatmessage()
    relay.receive_all()
    relay.assert_same_members()


def test_add_remove_and_stale_old_suddenly_sends(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    p0.immediate_create_group([p1, p2, p3])

    # p3 is offline and a member get deleted
    p0.send_del_member(p2.id)
    relay.receive_all([p0, p1, p2])

    # p3 sends a message with old memberlist and goes online
    p3.send_chatmessage()
    relay.receive_all()

    # p0 sends a message which should update all peers' members
    p0.send_chatmessage()
    relay.receive_all()

    relay.assert_same_members()
    assert p0.members == set([p0.id, p1.id, p3.id])
