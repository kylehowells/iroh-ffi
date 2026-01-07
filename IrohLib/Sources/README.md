# IrohLib Swift Sources

This directory contains Swift Package Manager executable targets for the iroh-ffi bindings.

## Directory Structure

| Directory | Description | Run Command |
|-----------|-------------|-------------|
| `IrohLib/` | Main Swift bindings library wrapping the iroh-ffi Rust library | (imported as dependency) |
| `GossipSender/` | Gossip demo - creates a topic and sends messages | `swift run GossipSender` |
| `GossipReceiver/` | Gossip demo - joins a topic and receives messages | `swift run GossipReceiver <TICKET>` |
| `GossipChat/` | Combined gossip demo (send/receive in one app) | `swift run GossipChat` |
| `GossipSenderSwift5/` | Swift 5 compatible gossip sender (completion handlers) | `swift run sender-swift5` |
| `BlobDemo/` | Blob transfer demo - add/share/download blobs | `swift run BlobDemo <command>` |
| `DocDemo/` | Document sync demo - create/join/sync documents | `swift run DocDemo <command>` |

## Source File Locations

Each executable target has a Swift source file in its directory. These correspond to reference copies in `examples/`:

| SPM Source | Examples Copy |
|------------|---------------|
| `GossipChat/GossipChat.swift` | `examples/GossipChat.swift` |
| `BlobDemo/main.swift` | `examples/BlobDemo.swift` |
| `DocDemo/main.swift` | `examples/DocDemo.swift` |

**Important:** The files in `IrohLib/Sources/*/` are the authoritative versions used by Swift Package Manager. The copies in `examples/` are for reference and documentation purposes. These should be kept in sync.

To verify sync status, run:
```bash
./examples/tests/verify_swift_sync.sh
```

## Demo Commands

### Gossip Messaging
```bash
# Terminal 1 - Start sender
swift run GossipSender

# Terminal 2 - Join with ticket
swift run GossipReceiver <TICKET>
```

### Blob Transfer
```bash
# Add and share data
swift run BlobDemo send-bytes "Hello, World!"
swift run BlobDemo send-file /path/to/file

# Download from ticket
swift run BlobDemo receive <TICKET>
```

### Document Sync
```bash
# Create a document
swift run DocDemo create

# Join existing document
swift run DocDemo join <TICKET>
```

## Building

From the `IrohLib/` directory:

```bash
# Build all targets
swift build

# Build release
swift build -c release

# Run a specific target
swift run <TargetName>
```

## Dependencies

All executable targets depend on `IrohLib`, which wraps the `Iroh.xcframework` containing the Rust FFI bindings.
