# Iroh FFI Demo Applications

Cross-language demo applications demonstrating Gossip, Blob, and Document functionality using iroh-ffi bindings.

## Demo Applications

| Protocol | Rust | Python | Swift |
|----------|------|--------|-------|
| **Gossip** | `gossip_chat.rs` | `gossip_chat.py` | `GossipChat.swift` |
| **Blobs** | `blob_demo.rs` | `blob_demo.py` | `BlobDemo.swift` |
| **Docs** | `doc_demo.rs` | `doc_demo.py` | `DocDemo.swift` |

## Cross-Language Interoperability

All combinations have been tested and verified to communicate successfully:

### Gossip Messaging

| Sender → Receiver | Rust | Python | Swift |
|-------------------|:----:|:------:|:-----:|
| **Rust**          | ✅   | ✅     | ✅    |
| **Python**        | ✅   | ✅     | ✅    |
| **Swift**         | ✅   | ✅     | ✅    |

### Blob Transfer

| Sender → Receiver | Rust | Python | Swift |
|-------------------|:----:|:------:|:-----:|
| **Rust**          | ✅   | ✅     | ✅    |
| **Python**        | ✅   | ✅     | ✅    |
| **Swift**         | ✅   | ✅     | ✅    |

### Document Sync

| Creator → Joiner | Rust | Python | Swift |
|------------------|:----:|:------:|:-----:|
| **Rust**         | ✅   | ✅     | ✅    |
| **Python**       | ✅   | ✅     | ✅    |
| **Swift**        | ✅   | ✅     | ✅    |

## Running the Demos

### Prerequisites

- **Rust**: `cargo build`
- **Python**: `maturin develop` (in virtualenv)
- **Swift**: `swift build` (in `IrohLib/` directory)

### Gossip Chat

Peer-to-peer messaging over a shared topic.

```bash
# Terminal 1 - Create a topic and wait for peers
cargo run --example gossip_chat -- send

# Terminal 2 - Join using the ticket from Terminal 1
cargo run --example gossip_chat -- receive <TICKET>

# Python
python examples/gossip_chat.py send
python examples/gossip_chat.py receive <TICKET>

# Swift (from IrohLib/)
swift run GossipSender
swift run GossipReceiver <TICKET>
```

### Blob Transfer

Transfer binary data between nodes using content-addressed storage.

```bash
# Terminal 1 - Add data and create a ticket
cargo run --example blob_demo -- send-bytes "Hello, World!"
cargo run --example blob_demo -- send-file /path/to/file

# Terminal 2 - Download using the ticket
cargo run --example blob_demo -- receive <TICKET>

# Python
python examples/blob_demo.py send-bytes "Hello, World!"
python examples/blob_demo.py receive <TICKET>

# Swift (from IrohLib/)
swift run BlobDemo send-bytes "Hello, World!"
swift run BlobDemo receive <TICKET>
```

### Document Sync

Collaborative document synchronization with real-time updates.

```bash
# Terminal 1 - Create a document
cargo run --example doc_demo -- create

# Terminal 2 - Join and sync using the ticket
cargo run --example doc_demo -- join <TICKET>

# Python
python examples/doc_demo.py create
python examples/doc_demo.py join <TICKET>

# Swift (from IrohLib/)
swift run DocDemo create
swift run DocDemo join <TICKET>
```

## Interactive Commands

All demos support interactive mode after initialization:

### Gossip Chat
- Type a message and press Enter to send
- `/quit` - Exit

### Blob Demo
- `list` - List all blobs
- `add <text>` - Add text as a blob
- `get <hash>` - Get blob by hash
- `/quit` - Exit

### Doc Demo
- `set <key> <value>` - Set a key-value pair
- `get <key>` - Get value by key
- `list` - List all entries
- `/quit` - Exit

## Automated Testing

The `tests/` directory contains automated cross-language integration tests:

```bash
# Run all tests
./examples/tests/run_all_tests.sh

# Run specific test suites
./examples/tests/run_all_tests.sh gossip   # Gossip messaging tests
./examples/tests/run_all_tests.sh blobs    # Blob transfer tests
./examples/tests/run_all_tests.sh docs     # Document sync tests
./examples/tests/run_all_tests.sh sync     # Verify Swift file sync

# Run individual test scripts
./examples/tests/test_gossip.sh
./examples/tests/test_blobs.sh
./examples/tests/test_docs.sh
./examples/tests/verify_swift_sync.sh
```

## External Compatibility

These demos are also compatible with native iroh applications using iroh 0.95:

- Tested with [iroh-blobs](https://github.com/n0-computer/iroh-blobs) examples
- Tested with standalone iroh gossip applications

## Verification Date

Last verified: 2026-01-07
