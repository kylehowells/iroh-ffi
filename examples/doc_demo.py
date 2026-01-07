#!/usr/bin/env python3
"""
Python document demo using iroh-ffi

Usage:
    python doc_demo.py create
    python doc_demo.py join [TICKET]

Examples:
    # Create a document and wait for peers
    python doc_demo.py create

    # Join an existing document
    python doc_demo.py join <TICKET>
"""

import asyncio
import sys
import os
import tempfile

# Add the parent directory to the path to find the iroh module
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import iroh
from iroh import (
    Iroh, NodeOptions, DocTicket, Query,
    LiveEventType, ShareMode, AddrInfoOptions,
    SubscribeCallback,
)


class DocCallback(SubscribeCallback):
    def __init__(self, name):
        self.name = name

    async def event(self, event):
        event_type = event.type()
        if event_type == LiveEventType.INSERT_LOCAL:
            entry = event.as_insert_local()
            key = entry.key().decode('utf-8', errors='replace')
            print(f"\n[{self.name}] Local insert: key='{key}', content_len={entry.content_len()}")
        elif event_type == LiveEventType.INSERT_REMOTE:
            insert = event.as_insert_remote()
            key = insert.entry.key().decode('utf-8', errors='replace')
            print(f"\n[{self.name}] Remote insert from {insert._from}: key='{key}', content_len={insert.entry.content_len()}")
        elif event_type == LiveEventType.CONTENT_READY:
            hash_val = event.as_content_ready()
            print(f"\n[{self.name}] Content ready: {hash_val}")
        elif event_type == LiveEventType.SYNC_FINISHED:
            sync = event.as_sync_finished()
            print(f"\n[{self.name}] Sync finished with peer: {sync.peer}")
        elif event_type == LiveEventType.NEIGHBOR_UP:
            peer = event.as_neighbor_up()
            print(f"\n[{self.name}] Neighbor up: {peer}")
        elif event_type == LiveEventType.NEIGHBOR_DOWN:
            peer = event.as_neighbor_down()
            print(f"\n[{self.name}] Neighbor down: {peer}")
        elif event_type == LiveEventType.PENDING_CONTENT_READY:
            print(f"\n[{self.name}] Pending content ready")

        print("> ", end="", flush=True)


async def create_and_host_doc(node, author):
    print("Creating new document...")
    doc = await node.docs().create()
    print(f"Document ID: {doc.id()}")

    # Subscribe to events
    callback = DocCallback("doc")
    await doc.subscribe(callback)
    print("Subscribed to document events")

    # Set some initial entries
    print("\nSetting initial entries...")
    await doc.set_bytes(author, b"greeting", b"Hello from Python!")
    await doc.set_bytes(author, b"count", b"0")
    print("Initial entries set")

    # Create and share ticket
    ticket = await doc.share(ShareMode.WRITE, AddrInfoOptions.RELAY_AND_ADDRESSES)

    print("\n=== DOC TICKET ===")
    print(str(ticket))
    print("==================\n")
    print("To join this document, run:")
    print(f"  python doc_demo.py join {ticket}")

    print("\n=== Interactive Mode ===")
    print("Commands:")
    print("  set <key> <value>  - Set a key-value pair")
    print("  get <key>          - Get a value by key")
    print("  list               - List all entries")
    print("  /quit              - Exit\n")

    # Interactive loop
    print("> ", end="", flush=True)

    try:
        while True:
            line = await asyncio.get_event_loop().run_in_executor(None, sys.stdin.readline)
            line = line.strip()

            if not line:
                print("> ", end="", flush=True)
                continue

            parts = line.split()
            cmd = parts[0] if parts else ""

            if cmd in ["/quit", "quit", "exit"]:
                break
            elif cmd == "set" and len(parts) >= 3:
                key = parts[1].encode('utf-8')
                value = " ".join(parts[2:]).encode('utf-8')
                hash_val = await doc.set_bytes(author, key, value)
                print(f"Set '{parts[1]}' = '{' '.join(parts[2:])}' (hash: {hash_val})")
            elif cmd == "get" and len(parts) >= 2:
                key = parts[1].encode('utf-8')
                query = Query.key_exact(key, None)
                entry = await doc.get_one(query)
                if entry:
                    content = await node.blobs().read_to_bytes(entry.content_hash())
                    value = content.decode('utf-8', errors='replace')
                    print(f"'{parts[1]}' = '{value}'")
                else:
                    print(f"Key '{parts[1]}' not found")
            elif cmd == "list":
                query = Query.all(None)
                entries = await doc.get_many(query)
                print(f"Entries ({len(entries)}):")
                for entry in entries:
                    key = entry.key().decode('utf-8', errors='replace')
                    content = await node.blobs().read_to_bytes(entry.content_hash())
                    value = content.decode('utf-8', errors='replace')
                    print(f"  '{key}' = '{value}'")
            else:
                print("Unknown command. Try: set, get, list, /quit")

            print("> ", end="", flush=True)

    except (KeyboardInterrupt, EOFError):
        pass


async def join_doc(node, author, ticket_str):
    print("Parsing ticket...")
    ticket = DocTicket(ticket_str)

    print("\nJoining document...")
    callback = DocCallback("doc")
    doc = await node.docs().join_and_subscribe(ticket, callback)
    print(f"Joined document: {doc.id()}")

    # Give it a moment to sync
    await asyncio.sleep(2)

    # List existing entries
    print("\nExisting entries:")
    query = Query.all(None)
    entries = await doc.get_many(query)
    for entry in entries:
        key = entry.key().decode('utf-8', errors='replace')
        content = await node.blobs().read_to_bytes(entry.content_hash())
        value = content.decode('utf-8', errors='replace')
        print(f"  '{key}' = '{value}'")

    print("\n=== Interactive Mode ===")
    print("Commands:")
    print("  set <key> <value>  - Set a key-value pair")
    print("  get <key>          - Get a value by key")
    print("  list               - List all entries")
    print("  /quit              - Exit\n")

    # Interactive loop
    print("> ", end="", flush=True)

    try:
        while True:
            line = await asyncio.get_event_loop().run_in_executor(None, sys.stdin.readline)
            line = line.strip()

            if not line:
                print("> ", end="", flush=True)
                continue

            parts = line.split()
            cmd = parts[0] if parts else ""

            if cmd in ["/quit", "quit", "exit"]:
                break
            elif cmd == "set" and len(parts) >= 3:
                key = parts[1].encode('utf-8')
                value = " ".join(parts[2:]).encode('utf-8')
                hash_val = await doc.set_bytes(author, key, value)
                print(f"Set '{parts[1]}' = '{' '.join(parts[2:])}' (hash: {hash_val})")
            elif cmd == "get" and len(parts) >= 2:
                key = parts[1].encode('utf-8')
                query = Query.key_exact(key, None)
                entry = await doc.get_one(query)
                if entry:
                    content = await node.blobs().read_to_bytes(entry.content_hash())
                    value = content.decode('utf-8', errors='replace')
                    print(f"'{parts[1]}' = '{value}'")
                else:
                    print(f"Key '{parts[1]}' not found")
            elif cmd == "list":
                query = Query.all(None)
                entries = await doc.get_many(query)
                print(f"Entries ({len(entries)}):")
                for entry in entries:
                    key = entry.key().decode('utf-8', errors='replace')
                    content = await node.blobs().read_to_bytes(entry.content_hash())
                    value = content.decode('utf-8', errors='replace')
                    print(f"  '{key}' = '{value}'")
            else:
                print("Unknown command. Try: set, get, list, /quit")

            print("> ", end="", flush=True)

    except (KeyboardInterrupt, EOFError):
        pass


def print_usage():
    print("Usage:")
    print("  python doc_demo.py create")
    print("  python doc_demo.py join [TICKET]")


async def main():
    args = sys.argv[1:]

    if len(args) < 1:
        print_usage()
        return

    command = args[0]

    # Setup event loop for callbacks
    iroh.iroh_ffi.uniffi_set_event_loop(asyncio.get_running_loop())

    # Create node with docs enabled in temp directory
    temp_dir = os.path.join(tempfile.gettempdir(), f"iroh-doc-py-{os.getpid()}")
    os.makedirs(temp_dir, exist_ok=True)

    print("=== Iroh Doc Demo (Python) ===\n")
    print(f"Creating node at {temp_dir}...")

    options = NodeOptions(enable_docs=True)
    node = await Iroh.persistent_with_options(temp_dir, options)

    # Wait for node to be online
    print("Waiting for node to come online...")
    await node.net().wait_online()

    my_id = node.net().node_id()
    my_addr = node.net().node_addr()
    print(f"Node ID: {my_id}")
    print(f"Relay URL: {my_addr.relay_url()}\n")

    # Get or create an author
    authors = await node.authors().list()
    if authors:
        author = authors[0]
        print(f"Using existing author: {author}")
    else:
        author = await node.authors().create()
        print(f"Created new author: {author}")

    if command == "create":
        await create_and_host_doc(node, author)
    elif command == "join":
        if len(args) < 2:
            print("Usage: python doc_demo.py join [TICKET]")
            return
        ticket_str = args[1]
        await join_doc(node, author, ticket_str)
    else:
        print_usage()
        return

    print("\nShutting down...")
    await node.node().shutdown()


if __name__ == "__main__":
    asyncio.run(main())
