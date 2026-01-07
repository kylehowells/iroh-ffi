#!/usr/bin/env python3
"""
Python gossip chat demo using iroh-ffi

Usage:
    python gossip_chat.py [TOPIC_HEX] [PEER_NODE_ID] [PEER_RELAY_URL]

If no arguments provided, creates a new topic and prints node info.
If TOPIC_HEX provided, joins that topic.
If PEER_NODE_ID and PEER_RELAY_URL provided, connects to that peer.
"""

import asyncio
import sys
import os
import tempfile

# Add the parent directory to the path to find the iroh module
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import iroh
from iroh import Iroh, MessageType, GossipMessageCallback, NodeAddr, PublicKey


class ChatCallback(GossipMessageCallback):
    def __init__(self, name):
        self.name = name

    async def on_message(self, msg):
        msg_type = msg.type()
        if msg_type == MessageType.NEIGHBOR_UP:
            peer = msg.as_neighbor_up()
            print(f"\n[{self.name}] Peer connected: {peer[:16]}...")
        elif msg_type == MessageType.NEIGHBOR_DOWN:
            peer = msg.as_neighbor_down()
            print(f"\n[{self.name}] Peer disconnected: {peer[:16]}...")
        elif msg_type == MessageType.RECEIVED:
            received = msg.as_received()
            content = received.content.decode('utf-8', errors='replace')
            print(f"\n[{self.name}] {received.delivered_from[:8]}...: {content}")
        elif msg_type == MessageType.LAGGED:
            print(f"\n[{self.name}] Warning: missed some messages")
        elif msg_type == MessageType.ERROR:
            err = msg.as_error()
            print(f"\n[{self.name}] Error: {err}")

        print("> ", end="", flush=True)


def hex_to_bytes(hex_str):
    return bytes.fromhex(hex_str)


def bytes_to_hex(data):
    return data.hex()


async def main():
    args = sys.argv[1:]

    print("=== Iroh Gossip Chat Demo (Python) ===\n")

    # Setup event loop for callbacks
    iroh.iroh_ffi.uniffi_set_event_loop(asyncio.get_running_loop())

    # Create node with persistent storage in temp directory
    temp_dir = os.path.join(tempfile.gettempdir(), f"iroh-chat-py-{os.getpid()}")
    os.makedirs(temp_dir, exist_ok=True)

    print(f"Creating node at {temp_dir}...")
    node = await Iroh.persistent(temp_dir)

    # Wait for node to be online
    print("Waiting for node to come online...")
    await node.net().wait_online()

    my_id = node.net().node_id()
    my_addr = node.net().node_addr()
    print(f"Node ID: {my_id}")
    print(f"Relay URL: {my_addr.relay_url()}")
    print(f"Direct addresses: {my_addr.direct_addresses()}\n")

    # Parse topic from args or generate random one
    if len(args) > 0:
        topic = bytearray(hex_to_bytes(args[0]))
    else:
        # Generate random topic
        import time
        import hashlib
        seed = f"{time.time()}-{os.getpid()}".encode()
        topic = bytearray(hashlib.sha256(seed).digest())

    if len(topic) != 32:
        print("Topic must be exactly 32 bytes (64 hex chars)")
        return

    print(f"Topic: {bytes_to_hex(topic)}")

    # If peer info provided, add it to discovery
    # Supports: TOPIC PEER_ID [RELAY_URL]
    bootstrap = []
    if len(args) > 1:
        peer_node_id = args[1]
        print(f"Adding peer: {peer_node_id[:16]}...")

        # If relay URL is also provided, add to StaticProvider for faster discovery
        if len(args) > 2:
            peer_relay_url = args[2]
            peer_pubkey = PublicKey.from_string(peer_node_id)
            peer_addr = NodeAddr(peer_pubkey, peer_relay_url, [])
            node.net().add_node_addr(peer_addr)
            print("Added peer with relay URL")
        else:
            print("Using discovery to find peer...")

        bootstrap = [peer_node_id]

    # Subscribe to gossip topic
    print("\nJoining gossip topic...")
    callback = ChatCallback("chat")
    sender = await node.gossip().subscribe(topic, bootstrap, callback)

    print("\n=== Chat started! Type messages and press Enter ===")
    print(f"Share this topic with others: {bytes_to_hex(topic)}")
    print(f"Share your node ID: {my_id}")
    relay_url = my_addr.relay_url()
    if relay_url:
        print(f"Share your relay URL: {relay_url}")
    print("\nCommands: /id, /quit\n")

    # Read from stdin and broadcast
    print("> ", end="", flush=True)

    try:
        while True:
            # Use asyncio to read from stdin
            line = await asyncio.get_event_loop().run_in_executor(None, sys.stdin.readline)
            line = line.strip()

            if not line:
                print("> ", end="", flush=True)
                continue

            if line == "/quit":
                break

            if line == "/id":
                print(f"Your ID: {my_id}")
                if relay_url:
                    print(f"Relay URL: {relay_url}")
                print("> ", end="", flush=True)
                continue

            # Broadcast message
            await sender.broadcast(line.encode('utf-8'))
            print("> ", end="", flush=True)

    except (KeyboardInterrupt, EOFError):
        pass

    print("\nShutting down...")
    await sender.cancel()
    await node.node().shutdown()


if __name__ == "__main__":
    asyncio.run(main())
