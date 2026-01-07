#!/usr/bin/env python3
"""
Python blob demo using iroh-ffi

Usage:
    python blob_demo.py send [FILE]
    python blob_demo.py receive [TICKET] [DEST_FILE]
    python blob_demo.py send-bytes [TEXT]

Examples:
    # Send a file
    python blob_demo.py send ./myfile.txt

    # Receive a file using a ticket
    python blob_demo.py receive <TICKET> ./downloaded.txt

    # Send some text as bytes
    python blob_demo.py send-bytes "Hello, iroh!"
"""

import asyncio
import sys
import os
import tempfile

# Add the parent directory to the path to find the iroh module
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import iroh
from iroh import (
    Iroh, BlobFormat, BlobTicket, SetTagOption, WrapOption,
    AddCallback, AddProgressType, DownloadCallback, DownloadProgressType,
    NodeAddr, PublicKey, AddrInfoOptions, BlobDownloadOptions
)


class PrintAddCallback(AddCallback):
    def __init__(self, name):
        self.name = name
        self.final_hash = None
        self.final_format = None

    async def progress(self, progress):
        progress_type = progress.type()
        if progress_type == AddProgressType.FOUND:
            found = progress.as_found()
            print(f"[{self.name}] Found: {found.name} ({found.size} bytes)")
        elif progress_type == AddProgressType.PROGRESS:
            p = progress.as_progress()
            print(f"\r[{self.name}] Progress: {p.offset} bytes", end="", flush=True)
        elif progress_type == AddProgressType.DONE:
            done = progress.as_done()
            print(f"\n[{self.name}] Done: {done.hash}")
        elif progress_type == AddProgressType.ALL_DONE:
            all_done = progress.as_all_done()
            self.final_hash = all_done.hash
            self.final_format = all_done.format
            print(f"[{self.name}] All done! Hash: {all_done.hash}, Format: {all_done.format}")
        elif progress_type == AddProgressType.ABORT:
            abort = progress.as_abort()
            print(f"[{self.name}] Aborted: {abort.error}")


class PrintDownloadCallback(DownloadCallback):
    def __init__(self, name):
        self.name = name

    async def progress(self, progress):
        progress_type = progress.type()
        if progress_type == DownloadProgressType.CONNECTED:
            print(f"[{self.name}] Connected to peer")
        elif progress_type == DownloadProgressType.FOUND:
            found = progress.as_found()
            print(f"[{self.name}] Found blob: {found.hash} ({found.size} bytes)")
        elif progress_type == DownloadProgressType.PROGRESS:
            p = progress.as_progress()
            print(f"\r[{self.name}] Download progress: {p.offset} bytes", end="", flush=True)
        elif progress_type == DownloadProgressType.DONE:
            print(f"\n[{self.name}] Blob download complete")
        elif progress_type == DownloadProgressType.ALL_DONE:
            all_done = progress.as_all_done()
            print(f"[{self.name}] All done! {all_done.bytes_written} bytes written, {all_done.bytes_read} bytes read")
        elif progress_type == DownloadProgressType.ABORT:
            abort = progress.as_abort()
            print(f"[{self.name}] Download aborted: {abort.error}")


def print_usage():
    print("Usage:")
    print("  python blob_demo.py send [FILE]")
    print("  python blob_demo.py receive [TICKET] [DEST_FILE]")
    print("  python blob_demo.py send-bytes [TEXT]")


async def send_file(node, file_path):
    """Add a file to blob store and create a ticket for sharing."""
    print(f"Adding file: {file_path}")

    abs_path = os.path.abspath(file_path)
    callback = PrintAddCallback("add")

    await node.blobs().add_from_path(
        abs_path,
        False,  # copy, not in-place
        SetTagOption.auto(),
        WrapOption.no_wrap(),
        callback
    )

    # After adding, list blobs to get the hash
    blobs = await node.blobs().list()
    if blobs:
        hash_obj = blobs[0]
        # Create a ticket for sharing
        ticket = await node.blobs().share(
            hash_obj,
            BlobFormat.RAW,
            AddrInfoOptions.ID
        )

        print("\n=== BLOB TICKET ===")
        print(str(ticket))
        print("===================\n")
        print("To download this file, run:")
        print(f"  python blob_demo.py receive {str(ticket)} [DEST_FILE]")


async def send_bytes(node, data):
    """Add bytes to blob store and create a ticket for sharing."""
    print(f"Adding {len(data)} bytes of data...")

    outcome = await node.blobs().add_bytes(data)

    print(f"Added blob with hash: {outcome.hash}")
    print(f"Format: {outcome.format}")
    print(f"Size: {outcome.size} bytes")

    # Create a ticket for sharing
    ticket = await node.blobs().share(
        outcome.hash,
        BlobFormat.RAW,
        AddrInfoOptions.ID
    )

    print("\n=== BLOB TICKET ===")
    print(str(ticket))
    print("===================\n")
    print("To download this blob, run:")
    print(f"  python blob_demo.py receive {str(ticket)} [DEST_FILE]")

    # Also show the content for text data
    try:
        text = data.decode('utf-8')
        print(f"\nBlob content: \"{text}\"")
    except:
        pass


async def receive_blob(node, ticket_str, dest_path):
    """Download a blob using a ticket and save to file."""
    print("Parsing ticket...")
    ticket = BlobTicket(ticket_str)

    hash_obj = ticket.hash()
    addr = ticket.node_addr()

    print(f"Blob hash: {hash_obj}")
    print(f"Provider relay URL: {addr.relay_url()}")

    # Add the node address to discovery for direct connection
    node.net().add_node_addr(addr)

    print("\nDownloading blob...")

    download_opts = BlobDownloadOptions(
        BlobFormat.RAW,
        [addr],
        SetTagOption.auto()
    )

    callback = PrintDownloadCallback("download")

    await node.blobs().download(hash_obj, download_opts, callback)

    print(f"\nExporting to file: {dest_path}")
    await node.blobs().write_to_path(hash_obj, dest_path)

    print(f"File saved to: {dest_path}")

    # If it's a small blob, show the content
    size = await node.blobs().size(hash_obj)
    if size < 1024:
        data = await node.blobs().read_to_bytes(hash_obj)
        try:
            text = bytes(data).decode('utf-8')
            print(f"\nBlob content: \"{text}\"")
        except:
            pass


async def main():
    args = sys.argv[1:]

    if len(args) < 1:
        print_usage()
        return

    command = args[0]

    print("=== Iroh Blob Demo (Python) ===\n")

    # Setup event loop for callbacks
    iroh.iroh_ffi.uniffi_set_event_loop(asyncio.get_running_loop())

    # Create node with persistent storage in temp directory
    temp_dir = os.path.join(tempfile.gettempdir(), f"iroh-blob-py-{os.getpid()}")
    os.makedirs(temp_dir, exist_ok=True)

    print(f"Creating node at {temp_dir}...")
    node = await Iroh.persistent(temp_dir)

    # Wait for node to be online
    print("Waiting for node to come online...")
    await node.net().wait_online()

    my_id = node.net().node_id()
    my_addr = node.net().node_addr()
    print(f"Node ID: {my_id}")
    print(f"Relay URL: {my_addr.relay_url()}\n")

    try:
        if command == "send":
            if len(args) < 2:
                print("Usage: python blob_demo.py send [FILE]")
                return
            file_path = args[1]
            await send_file(node, file_path)

            print("\nWaiting for peer to download... Press Ctrl+C to exit.")
            await asyncio.Event().wait()

        elif command == "send-bytes":
            if len(args) < 2:
                print("Usage: python blob_demo.py send-bytes [TEXT]")
                return
            text = " ".join(args[1:])
            await send_bytes(node, text.encode('utf-8'))

            print("\nWaiting for peer to download... Press Ctrl+C to exit.")
            await asyncio.Event().wait()

        elif command == "receive":
            if len(args) < 3:
                print("Usage: python blob_demo.py receive [TICKET] [DEST_FILE]")
                return
            ticket_str = args[1]
            dest_path = args[2]
            await receive_blob(node, ticket_str, dest_path)

        else:
            print_usage()

    except KeyboardInterrupt:
        pass

    print("\nShutting down...")
    await node.node().shutdown()


if __name__ == "__main__":
    asyncio.run(main())
