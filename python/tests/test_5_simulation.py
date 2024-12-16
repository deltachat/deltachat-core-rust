import pytest


@pytest.fixture
def relay():
    return Relay()


def repr_peers(peers):
    return ",".join(p.id for p in peers)


class Relay:
    def __init__(self):
        self.peers = []

    def make_peers(self, num):
        newpeers = [Peer(self, i) for i in range(num)]
        self.peers.extend(newpeers)
        return newpeers

    def receive_all(self, peers=None):
        peers = peers if peers else self.peers
        for peer in peers:
            for from_peer in self.peers:
                drain_mailbox(peer, from_peer)

    def assert_same_members(self):
        for peer1, peer2 in zip(self.peers, self.peers[1:]):
            assert peer1.members == peer2.members
            nums = repr_peers(peer1.members)
            print(f"{peer1.id} and {peer2.id} have same members {nums}")


class Message:
    def __init__(self, sender, **payload):
        self.sender = sender
        self.recipients = list(sender.members)
        self.payload = payload

    def __repr__(self):
        nums = repr_peers(self.recipients)
        return f"<{self.__class__.__name__} {self.sender.id}->{nums} {self.payload}"


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
        peer.members.update(self.recipients)


class Peer:
    """A peer in a group"""

    def __init__(self, relay, num):
        self.relay = relay
        self.id = f"p{num}"
        self.members = set()
        self.from2mailbox = {}

    def __eq__(self, other):
        return self.id == other.id

    def __hash__(self):
        return int(self.id[1:])

    def __repr__(self):
        return f"<Peer {self.id} members={repr_peers(self.members)}>"

    def immediate_create_group(self, peers):
        assert not self.members
        self.members.add(self)
        for peer in peers:
            self.add_member(peer)
        self.relay.receive_all()

    def add_member(self, newmember):
        self.members.add(newmember)
        message = AddMemberMessage(self, newmember=newmember)
        self.queue_message(message)

    def del_member(self, member):
        message = DelMemberMessage(self, member=member)
        self.queue_message(message)
        self.members.remove(member)

    def send_chatmessage(self):
        message = ChatMessage(self)
        self.queue_message(message)

    def queue_message(self, message):
        for peer in self.members:
            peer.from2mailbox.setdefault(self, []).append(message)


### processing group membership message

def drain_mailbox(peer, from_peer):
    for msg in peer.from2mailbox.get(from_peer, []):
        msg.receive_imf(peer)



### Tests


def test_add_and_remove(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    # create group
    p0.immediate_create_group([p1])
    assert p0.members == p1.members == set([p0, p1])

    # add members
    p0.add_member(p2)
    p0.add_member(p3)
    relay.receive_all()
    relay.assert_same_members()

    p3.del_member(p0)
    relay.receive_all()
    relay.assert_same_members()


def test_concurrent_add(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    p0.immediate_create_group([p1])
    # concurrent adding and then let base set send a chat message
    p1.add_member(p2)
    p0.add_member(p3)

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
    p0.del_member(p2)
    relay.receive_all([p0, p1, p2])

    # p3 sends a message with old memberlist and goes online
    p3.send_chatmessage()
    relay.receive_all()

    # p0 sends a message which should update all peers' members
    p0.send_chatmessage()
    relay.receive_all()

    relay.assert_same_members()
    assert p0.members == set([p0, p1, p3])
