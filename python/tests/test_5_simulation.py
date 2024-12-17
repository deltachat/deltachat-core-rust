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

    def dump(self, title):
        print()
        print(f"# {title}")
        for peer_id, peer in self.peers.items():
            pending = sum(len(x) for x in peer.from2mailbox.values())
            members = ",".join(peer.members)
            print(f"{peer_id} clock={peer.current_clock} members={members} pending={pending}")
        print()

    def receive_all(self, peers=None):
        peers = peers if peers is not None else list(self.peers.values())
        for peer in peers:
            # drain peer mailbox by reading messages from each sender separately
            for from_peer in self.peers.values():
                pending = peer.from2mailbox.pop(from_peer, [])
                if from_peer.id != peer.id:
                    for msg in pending:
                        msg.receive(peer)

    def assert_group_consistency(self):
        peers = list(self.peers.values())
        for peer1, peer2 in zip(peers, peers[1:]):
            assert peer1.members == peer2.members
            assert peer1.current_clock == peer2.current_clock
            nums = ",".join(sorted(peer1.members))
            print(f"{peer1.id} and {peer2.id} have same members {nums}")


class Message:
    def __init__(self, sender, **payload):
        self.sender = sender
        self.payload = payload
        self.recipients = set(sender.members)
        # we increment clock on AddMemberMessage and DelMemberMessage
        sender.current_clock += self.clock_inc
        self.clock = sender.current_clock
        self.send()

    def __repr__(self):
        nums = ",".join(self.recipients)
        return f"<{self.__class__.__name__} clock={self.clock} {self.sender.id}->{nums} {self.payload}>"

    def send(self):
        print(f"sending {self}")
        for peer_id in self.sender.members:
            peer = self.sender.relay.peers[peer_id]
            peer.from2mailbox.setdefault(self.sender, []).append(self)


class AddMemberMessage(Message):
    clock_inc = 1

    def __init__(self, sender, member):
        sender.members.add(member)
        super().__init__(sender, member=member)

    def receive(self, peer):
        if not peer.members:  # create group
            peer.members = self.recipients.copy()
            peer.current_clock = self.clock
            return

        peer.members.add(self.payload["member"])
        if peer.current_clock < self.clock:
            peer.members.update(self.recipients)
            peer.current_clock = self.clock
        elif peer.current_clock == self.clock:
            if peer.members != self.recipients:
                peer.current_clock += 1


class DelMemberMessage(Message):
    clock_inc = 1

    def send(self):
        super().send()
        self.sender.members.remove(self.payload["member"])

    def receive(self, peer):
        member = self.payload["member"]
        if member in peer.members:
            if peer.current_clock <= self.clock:
                peer.members.remove(member)
            peer.current_clock = self.clock


class ChatMessage(Message):
    clock_inc = 0

    def receive(self, peer):
        print(f"receive {peer.id} clock={peer.current_clock} msgclock={self.clock}")
        if peer.current_clock < self.clock:
            print(f"{peer.id} is outdated, using incoming memberslist")
            peer.members = set(self.recipients)
            peer.current_clock = self.clock
            print(f"-> NEWCLOCK: {peer.current_clock}")
        elif peer.current_clock == self.clock:
            if peer.members != set(self.recipients):
                print(f"{peer.id} has different members than incoming same-clock message")
                print(f"{peer.id} resetting to incoming recipients, and increase own clock")
                peer.members = set(self.recipients)
                peer.current_clock = self.clock + 1
        else:
            print(f"{peer.id} has newer clock than incoming message")


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
            AddMemberMessage(self, member=peer.id)
        self.relay.receive_all()


### Tests


def test_add_and_remove(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    # create group
    p0.immediate_create_group([p1])
    assert p0.members == p1.members == set([p0.id, p1.id])

    # add members
    AddMemberMessage(p0, member=p2.id)
    AddMemberMessage(p0, member=p3.id)
    relay.receive_all()
    relay.assert_group_consistency()

    DelMemberMessage(p3, member=p0.id)
    relay.receive_all()
    relay.assert_group_consistency()


def test_concurrent_add(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    p0.immediate_create_group([p1])
    # concurrent adding and then let base set send a chat message
    AddMemberMessage(p1, member=p2.id)
    AddMemberMessage(p0, member=p3.id)
    relay.receive_all()

    relay.dump("after concurrent add")
    # only now do p0 and p1 know of each others additions
    # so p0 or p1 needs to send another message to get consistent membership
    ChatMessage(p0)
    relay.receive_all()
    relay.dump("after chatmessage")
    relay.assert_group_consistency()


def test_add_remove_and_stale_member_sends_chatmessage(relay):
    p0, p1, p2, p3 = relay.make_peers(4)

    p0.immediate_create_group([p1, p2, p3])

    # p3 is offline and p0 deletes p2
    DelMemberMessage(p0, member=p2.id)
    relay.receive_all([p0, p1, p2])

    # p3 sends a message with old memberlist and goes online
    ChatMessage(p3)
    relay.receive_all()
    relay.assert_group_consistency()
    ChatMessage(p0)
    relay.receive_all()
    assert p0.members == set(["p0", "p1", "p3"])


def test_add_remove_and_stale_member_sends_addition(relay):
    p0, p1, p2, p3, p4 = relay.make_peers(5)

    p0.immediate_create_group([p1, p2, p3])

    # p3 is offline and p0 deletes p2
    DelMemberMessage(p0, member=p2.id)
    relay.receive_all([p0, p1, p2])

    # p3 sends a message with member addition and goes online
    AddMemberMessage(p3, member=p4.id)
    relay.receive_all()
    relay.dump("after p3 is online")

    # we need a chat message from a higher clock state to heal consistency
    ChatMessage(p0)
    relay.receive_all()
    relay.dump("after p0 sent chatmessage")
    relay.assert_group_consistency()
    assert p0.members == set(["p0", "p1", "p3", "p4"])
