/// Swift gossip chat demo using iroh-ffi
///
/// Usage:
///   swift run GossipChat [TOPIC_HEX] [PEER_NODE_ID] [PEER_RELAY_URL]
///
/// If no arguments provided, creates a new topic and prints node info.
/// If TOPIC_HEX provided, joins that topic.
/// If PEER_NODE_ID and PEER_RELAY_URL provided, connects to that peer.

import Foundation
import IrohLib

class ChatCallback: GossipMessageCallback {
    let name: String

    init(name: String) {
        self.name = name
    }

    func onMessage(msg: Message) async throws {
        switch msg.type() {
        case .neighborUp:
            let peer = msg.asNeighborUp()
            print("\n[\(name)] Peer connected: \(String(peer.prefix(16)))...")
        case .neighborDown:
            let peer = msg.asNeighborDown()
            print("\n[\(name)] Peer disconnected: \(String(peer.prefix(16)))...")
        case .received:
            let received = msg.asReceived()
            let content = String(data: Data(received.content), encoding: .utf8) ?? "<binary>"
            print("\n[\(name)] \(String(received.deliveredFrom.prefix(8)))...: \(content)")
        case .lagged:
            print("\n[\(name)] Warning: missed some messages")
        case .error:
            let err = msg.asError()
            print("\n[\(name)] Error: \(err)")
        }
        print("> ", terminator: "")
        fflush(stdout)
    }
}

func hexToBytes(_ hex: String) -> [UInt8] {
    var bytes = [UInt8]()
    var index = hex.startIndex
    while index < hex.endIndex {
        let nextIndex = hex.index(index, offsetBy: 2)
        if let byte = UInt8(hex[index..<nextIndex], radix: 16) {
            bytes.append(byte)
        }
        index = nextIndex
    }
    return bytes
}

func bytesToHex(_ bytes: [UInt8]) -> String {
    return bytes.map { String(format: "%02x", $0) }.joined()
}

@main
struct GossipChat {
    static func main() async throws {
        let args = Array(CommandLine.arguments.dropFirst())

        print("=== Iroh Gossip Chat Demo (Swift) ===\n")

        // Create node with persistent storage in temp directory
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("iroh-chat-swift-\(ProcessInfo.processInfo.processIdentifier)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

        print("Creating node at \(tempDir.path)...")
        let node = try await Iroh.persistent(path: tempDir.path)

        // Wait for node to be online
        print("Waiting for node to come online...")
        try await node.net().waitOnline()

        let myId = node.net().nodeId()
        let myAddr = node.net().nodeAddr()
        print("Node ID: \(myId)")
        print("Relay URL: \(myAddr.relayUrl() ?? "none")")
        print("Direct addresses: \(myAddr.directAddresses())\n")

        // Parse topic from args or generate random one
        var topic: [UInt8]
        if args.count > 0 {
            topic = hexToBytes(args[0])
        } else {
            // Generate random topic
            var randomBytes = [UInt8](repeating: 0, count: 32)
            _ = SecRandomCopyBytes(kSecRandomDefault, 32, &randomBytes)
            topic = randomBytes
        }

        guard topic.count == 32 else {
            print("Topic must be exactly 32 bytes (64 hex chars)")
            return
        }

        print("Topic: \(bytesToHex(topic))")

        // If peer info provided, add it to discovery
        // Supports: TOPIC PEER_ID [RELAY_URL]
        var bootstrap: [String] = []
        if args.count > 1 {
            let peerNodeId = args[1]
            print("Adding peer: \(String(peerNodeId.prefix(16)))...")

            // If relay URL is also provided, add to StaticProvider for faster discovery
            if args.count > 2 {
                let peerRelayUrl = args[2]
                let peerPubkey = try PublicKey.fromString(s: peerNodeId)
                let peerAddr = NodeAddr(nodeId: peerPubkey, derpUrl: peerRelayUrl, addresses: [])
                try node.net().addNodeAddr(nodeAddr: peerAddr)
                print("Added peer with relay URL")
            } else {
                print("Using discovery to find peer...")
            }

            bootstrap = [peerNodeId]
        }

        // Subscribe to gossip topic
        print("\nJoining gossip topic...")
        let callback = ChatCallback(name: "chat")
        let sender = try await node.gossip().subscribe(
            topic: Data(topic),
            bootstrap: bootstrap,
            cb: callback
        )

        print("\n=== Chat started! Type messages and press Enter ===")
        print("Share this topic with others: \(bytesToHex(topic))")
        print("Share your node ID: \(myId)")
        if let relayUrl = myAddr.relayUrl() {
            print("Share your relay URL: \(relayUrl)")
        }
        print("\nCommands: /id, /quit\n")

        // Read from stdin and broadcast
        print("> ", terminator: "")
        fflush(stdout)

        while let line = readLine() {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)

            if trimmed.isEmpty {
                print("> ", terminator: "")
                fflush(stdout)
                continue
            }

            if trimmed == "/quit" {
                break
            }

            if trimmed == "/id" {
                print("Your ID: \(myId)")
                if let relayUrl = myAddr.relayUrl() {
                    print("Relay URL: \(relayUrl)")
                }
                print("> ", terminator: "")
                fflush(stdout)
                continue
            }

            // Broadcast message
            try await sender.broadcast(msg: Data(trimmed.utf8))
            print("> ", terminator: "")
            fflush(stdout)
        }

        print("\nShutting down...")
        try await sender.cancel()
        try await node.node().shutdown()
    }
}
