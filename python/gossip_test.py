# tests that correspond to the `src/gossip.rs` rust api
import pytest
import asyncio
import iroh

from iroh import Iroh, MessageType, GossipMessageCallback

class Callback(GossipMessageCallback):
    def __init__(self, name):
        print("init", name)
        self.name = name
        self.chan = asyncio.Queue()

    async def on_message(self, msg):
        print(self.name, msg.type())
        await self.chan.put(msg)

@pytest.mark.asyncio
async def test_gossip_basic():
    # setup event loop, to ensure async callbacks work
    iroh.iroh_ffi.uniffi_set_event_loop(asyncio.get_running_loop())

    n0 = await Iroh.memory()
    n1 = await Iroh.memory()

    # Wait for nodes to be online (connected to relay and have addresses)
    await n0.net().wait_online()
    await n1.net().wait_online()

    # Create a topic
    topic = bytearray([1] * 32)

    # Get addresses and IDs for both nodes (sync in 0.95)
    n0_id = n0.net().node_id()
    n0_addr = n0.net().node_addr()
    n1_id = n1.net().node_id()
    n1_addr = n1.net().node_addr()

    print(f"n0 addr: {n0_addr}")
    print(f"n1 addr: {n1_addr}")

    # Add peer addresses to StaticProvider for discovery
    n0.net().add_node_addr(n1_addr)
    n1.net().add_node_addr(n0_addr)

    # n0 subscribes with empty bootstrap
    cb0 = Callback("n0")
    print("subscribe n0")
    sink0 = await n0.gossip().subscribe(topic, [], cb0)

    # n1 subscribes with n0 as bootstrap
    cb1 = Callback("n1")
    print("subscribe n1")
    sink1 = await n1.gossip().subscribe(topic, [n0_id], cb1)

    # Wait for n0 to see n1 as a neighbor
    while (True):
        event = await asyncio.wait_for(cb0.chan.get(), timeout=10.0)
        print("n0 <<", event.type())
        if (event.type() == MessageType.NEIGHBOR_UP):
            break

    # Give gossip time to establish
    await asyncio.sleep(0.5)

    # Broadcast message from node 0
    print("broadcasting message")
    msg_content = bytearray("hello".encode("utf-8"))
    await sink0.broadcast(msg_content)

    # Wait for the message on node 1
    found = False
    while (True):
        event = await asyncio.wait_for(cb1.chan.get(), timeout=10.0)
        print("n1 <<", event.type())
        if (event.type() == MessageType.RECEIVED):
            msg = event.as_received()
            assert msg.content == msg_content
            assert msg.delivered_from == n0_id
            found = True
            break

    assert found

    await sink0.cancel()
    await sink1.cancel()

    await n0.node().shutdown()
    await n1.node().shutdown()
