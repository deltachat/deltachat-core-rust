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
        newpeers = [Peer(relay, i) for i in range(num)]
        self.peers.extend(newpeers)
        return newpeers

    def receive_all(self):
        for peer in self.peers:
            for from_peer in self.peers:
                drain_mailbox(peer, from_peer)

    def assert_same_members(self):
        for peer1, peer2 in zip(self.peers, self.peers[1:]):
            assert peer1.members == peer2.members
            nums = repr_peers(peer1.members)
            print(f"{peer1.id} and {peer2.id} have same members {nums}")


class Message:
    def __init__(self, sender, recipients, msgtype, payload):
        self.sender = sender
        self.recipients = recipients
        self.msgtype = msgtype
        self.payload = payload

    def __repr__(self):
        nums = repr_peers(self.recipients)
        return f"<Message {self.sender.id}->{nums} {self.msgtype} {self.payload}"


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

    def add_member(self, newmember):
        self.members.add(newmember)
        self.queue_message(msgtype="addmember", newmember=newmember)

    def del_member(self, member):
        self.queue_message(msgtype="delmember", member=member)
        self.members.remove(member)

    def send_chatmessage(self):
        self.queue_message(msgtype="chatmessage")

    def queue_message(self, msgtype, **payload):
        msg = Message(self, list(self.members), msgtype, payload)
        for peer in self.members:
            peer.from2mailbox.setdefault(self, []).append(msg)


def create_group(peers):
    for peer in peers:
        peer.members.update(peers)


### Naive Algorithm for processing group membership message


def drain_mailbox(peer, from_peer):
    for msg in peer.from2mailbox.get(from_peer, []):
        if msg.msgtype == "addmember":
            peer.members.add(msg.payload["newmember"])
            peer.members.update(msg.recipients)
        elif msg.msgtype == "delmember":
            member = msg.payload["member"]
            if member in peer.members:
                peer.members.remove(member)
        elif msg.msgtype == "chatmessage":
            peer.members.update(msg.recipients)


### Tests


def test_add_and_remove(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    create_group([p0, p1])
    assert p0.members == p1.members
    assert not p2.members and not p3.members

    p0.add_member(p2)
    p0.add_member(p3)
    relay.receive_all()
    relay.assert_same_members()

    p3.del_member(p0)
    relay.receive_all()
    relay.assert_same_members()


def test_concurrent_add(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    create_group([p0, p1])
    assert not p2.members and not p3.members
    # concurrent adding and then let base set send a chat message
    p1.add_member(p2)
    p0.add_member(p3)

    # now p0 and p1 send a regular message
    p0.send_chatmessage()
    p1.send_chatmessage()
    relay.receive_all()
    p0.send_chatmessage()
    relay.receive_all()
    relay.assert_same_members()
