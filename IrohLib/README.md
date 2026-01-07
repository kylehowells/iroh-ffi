# IrohLib

Swift bindings for [Iroh](https://iroh.computer) - a toolkit for building distributed applications.

IrohLib provides Swift access to Iroh's networking capabilities including:
- **Gossip Protocol** - Pub/sub messaging between peers
- **Blob Storage** - Content-addressed data storage and transfer
- **Documents** - Collaborative key-value stores with sync
- **Networking** - Direct peer-to-peer connections with NAT traversal

## Requirements

- macOS 12+ / iOS 15+
- Swift 5.8+
- Xcode 14+

## Installation

### Swift Package Manager

Add IrohLib to your `Package.swift`:

```swift
dependencies: [
    .package(path: "../IrohLib")  // Local path
    // Or from GitHub:
    // .package(url: "https://github.com/kylehowells/iroh-ffi", branch: "main")
]
```

Then add it to your target:

```swift
.target(
    name: "YourApp",
    dependencies: ["IrohLib"]
)
```

## Quick Start

```swift
import IrohLib

// Create a node
let node = try await Iroh.memory()

// Get your endpoint ID (share this with peers)
let myId = node.net().nodeId()
print("My Endpoint ID: \(myId)")

// Subscribe to gossip topic
let topic = Data(repeating: 0, count: 32)  // 32-byte topic ID
let sender = try await node.gossip().subscribe(
    topic: topic,
    bootstrap: [],
    cb: MyCallback()
)

// Send a message
try await sender.broadcast(msg: Data("Hello!".utf8))
```

## Building

```bash
cd IrohLib
swift build
```

## Demo Applications

IrohLib includes several demo applications demonstrating different usage patterns.

### GossipChat (Interactive)

An interactive gossip chat application using async/await.

```bash
swift run GossipChat
```

Features:
- Interactive command-line chat
- Join topics with `/join <topic>`
- Send messages by typing and pressing Enter
- See peer connections in real-time

### GossipReceiver (CLI)

A simple receiver that listens for gossip messages on a channel.

```bash
swift run receiver [--channel <name>]
```

**Options:**
- `--channel` - Channel/topic name (default: "chat")

**Example:**
```bash
$ swift run receiver --channel chat
Receiver running
Endpoint ID: 8987d8fe2eb861f723d47c48b4b43682f0efde29e596ac4bade4dad0b8c1e9c7
Listening on channel `chat`...

:: Peer connected: f44ed299bf
[f44ed299bf] > Hello from another peer!
:: Peer disconnected: f44ed299bf
```

### GossipSender (CLI)

Sends a single gossip message to a peer.

```bash
swift run sender --peer <endpoint_id> [--channel <name>] <message>
```

**Options:**
- `--peer` - Endpoint ID of the receiver to connect to (required)
- `--channel` - Channel/topic name (default: "chat")
- `<message>` - Message to send

**Example:**
```bash
$ swift run sender --peer 8987d8fe2eb861f7... --channel chat "Hello World!"
Connecting to peer 8987d8fe2e...
Waiting for peer connection...
Connected!
Message sent: Hello World!
Done.
```

### GossipSenderSwift5 (Completion Handlers)

A Swift 5 compatible sender using completion handlers and delegates instead of async/await.

```bash
swift run sender-swift5 <peer_id> [message]
```

**Example:**
```bash
$ swift run sender-swift5 8987d8fe2eb861f7... "Hello from Swift 5!"
Starting node...
Waiting for node to come online...
Connecting to peer 8987d8fe2e...
Subscribed, waiting for peer connection...
:: Peer connected: 8987d8fe2e
Connected! Sending message...
Message sent: Hello from Swift 5!
Done.
```

## API Patterns

### Async/Await (Swift 5.5+)

The primary API uses Swift's modern concurrency:

```swift
// Create node
let node = try await Iroh.persistent(path: "/path/to/storage")

// Wait for network
try await node.net().waitOnline()

// Subscribe to gossip
let sender = try await node.gossip().subscribe(
    topic: topicData,
    bootstrap: [peerEndpointId],
    cb: callback
)

// Send message
try await sender.broadcast(msg: messageData)
```

### Completion Handlers (Swift 5.0+)

For compatibility with older Swift or non-async contexts, wrap the APIs:

```swift
extension Iroh {
    static func persistent(
        path: String,
        queue: DispatchQueue = .main,
        completion: @escaping (Result<Iroh, Error>) -> Void
    ) {
        Task {
            do {
                let node = try await Iroh.persistent(path: path)
                queue.async { completion(.success(node)) }
            } catch {
                queue.async { completion(.failure(error)) }
            }
        }
    }
}

// Usage
Iroh.persistent(path: "/tmp/iroh") { result in
    switch result {
    case .success(let node):
        print("Node created!")
    case .failure(let error):
        print("Error: \(error)")
    }
}
```

### Delegate Pattern

For receiving gossip messages, implement `GossipMessageCallback`:

```swift
class MyCallback: GossipMessageCallback {
    func onMessage(msg: Message) async throws {
        switch msg.type() {
        case .received:
            let received = msg.asReceived()
            let content = String(data: Data(received.content), encoding: .utf8)
            print("Got message: \(content ?? "<binary>")")
        case .neighborUp:
            print("Peer connected: \(msg.asNeighborUp())")
        case .neighborDown:
            print("Peer disconnected: \(msg.asNeighborDown())")
        case .error:
            print("Error: \(msg.asError())")
        case .lagged:
            print("Warning: message queue lagged")
        }
    }
}
```

Or use a delegate adapter for non-async code (see `GossipSenderSwift5` for full example):

```swift
protocol GossipDelegate: AnyObject {
    func gossipDidReceiveMessage(_ content: Data, from peer: String)
    func gossipPeerDidConnect(_ peer: String)
    func gossipPeerDidDisconnect(_ peer: String)
    func gossipDidEncounterError(_ error: String)
}
```

## Topic IDs

Topics are 32-byte identifiers. To ensure compatibility with other Iroh applications, use blake3 hashing:

```python
# Python
import blake3
topic = blake3.blake3(b"chat").digest()
```

```rust
// Rust
let topic = blake3::hash(b"chat");
```

The demos use a pre-computed hash for the "chat" channel:
```
blake3("chat") = 504c1dbb87fc1cd93594bd6baad1b520229bd222e16d9c48138998f602993c67
```

## Cross-Language Compatibility

These Swift demos are fully compatible with:
- Rust applications using `iroh` 0.95
- Python applications using `iroh-ffi`
- The [GossipDemo](https://github.com/example/GossipDemo) reference implementation

## Building from Source

IrohLib requires a pre-built `Iroh.xcframework` containing the compiled Rust library. This is not included in the git repository due to its size (~280MB per architecture).

### Prerequisites

- Rust toolchain (`rustup`)
- iOS and macOS targets:
  ```bash
  rustup target add aarch64-apple-ios
  rustup target add aarch64-apple-ios-sim
  rustup target add x86_64-apple-ios
  rustup target add aarch64-apple-darwin
  ```

### Build the xcframework

From the repository root:

```bash
./make_swift.sh
```

This will:
1. Compile the Rust library for all Apple platforms
2. Generate the xcframework in `IrohLib/artifacts/Iroh.xcframework`
3. Generate Swift bindings via UniFFI

After building, you can use IrohLib as a local package dependency.

## Publishing Binary Releases

To allow users to consume IrohLib without building from source, publish the xcframework as a GitHub release.

### 1. Build and zip the xcframework

```bash
./make_swift.sh
cd IrohLib/artifacts
zip -r Iroh.xcframework.zip Iroh.xcframework
shasum -a 256 Iroh.xcframework.zip
# Note the checksum output
```

### 2. Create GitHub release

1. Create a new release on GitHub (e.g., `v0.95.0`)
2. Attach `Iroh.xcframework.zip` to the release

### 3. Update Package.swift for binary distribution

Change the binary target from local path to URL:

```swift
// Before (local path - for development)
.binaryTarget(
    name: "Iroh",
    path: "artifacts/Iroh.xcframework")

// After (remote URL - for distribution)
.binaryTarget(
    name: "Iroh",
    url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.95.0/Iroh.xcframework.zip",
    checksum: "YOUR_SHA256_CHECKSUM_HERE")
```

### 4. Tag and push

```bash
git add IrohLib/Package.swift
git commit -m "Release v0.95.0 with binary xcframework"
git tag v0.95.0
git push origin main --tags
```

Now users can add IrohLib directly from GitHub:

```swift
dependencies: [
    .package(url: "https://github.com/kylehowells/iroh-ffi", from: "0.95.0")
]
```

## License

MIT OR Apache-2.0
