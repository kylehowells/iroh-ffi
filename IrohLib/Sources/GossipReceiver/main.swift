/// Swift gossip receiver - compatible with GossipDemo
///
/// Usage:
///   receiver [--channel <name>]
///
/// Listens for gossip messages on a channel (default: "chat")

import Foundation
import IrohLib
import ArgumentParser

/// Convert a channel name to a 32-byte topic ID using blake3
/// Uses Python blake3 via shell command for compatibility
func topicFromName(_ name: String) -> Data {
    // Pre-computed hash for common channel
    if name == "chat" {
        return Data(hexString: "504c1dbb87fc1cd93594bd6baad1b520229bd222e16d9c48138998f602993c67")!
    }

    // For other channels, try to compute via Python
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
    process.arguments = ["python3", "-c", "import blake3; print(blake3.blake3(b'\(name)').hexdigest())"]

    let pipe = Pipe()
    process.standardOutput = pipe
    process.standardError = FileHandle.nullDevice

    do {
        try process.run()
        process.waitUntilExit()

        if process.terminationStatus == 0 {
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let hex = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
               let result = Data(hexString: hex) {
                return result
            }
        }
    } catch {
        // Fall through to error
    }

    fatalError("Failed to compute blake3 hash for channel '\(name)'. Install blake3: pip3 install blake3")
}

extension Data {
    init?(hexString: String) {
        let len = hexString.count / 2
        var data = Data(capacity: len)
        var index = hexString.startIndex
        for _ in 0..<len {
            let nextIndex = hexString.index(index, offsetBy: 2)
            guard let byte = UInt8(hexString[index..<nextIndex], radix: 16) else { return nil }
            data.append(byte)
            index = nextIndex
        }
        self = data
    }
}

/// Print with immediate flush
func printFlush(_ message: String) {
    print(message)
    fflush(stdout)
}

/// Callback handler for gossip messages
class ReceiverCallback: GossipMessageCallback {
    func onMessage(msg: Message) async throws {
        switch msg.type() {
        case .neighborUp:
            let peer = msg.asNeighborUp()
            printFlush(":: Peer connected: \(String(peer.prefix(10)))")
        case .neighborDown:
            let peer = msg.asNeighborDown()
            printFlush(":: Peer disconnected: \(String(peer.prefix(10)))")
        case .received:
            let received = msg.asReceived()
            let content = String(data: Data(received.content), encoding: .utf8) ?? "<binary>"
            let from = String(received.deliveredFrom.prefix(10))
            printFlush("[\(from)] > \(content)")
        case .lagged:
            printFlush(":: Warning: message queue lagged")
        case .error:
            let err = msg.asError()
            printFlush(":: Error: \(err)")
        }
    }
}

@main
struct Receiver: AsyncParsableCommand {
    static var configuration = CommandConfiguration(
        commandName: "receiver",
        abstract: "Listen for gossip messages on a topic"
    )

    @Option(name: .long, help: "Channel/topic name to subscribe to")
    var channel: String = "chat"

    mutating func run() async throws {
        // Create node with persistent storage in temp directory
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("iroh-receiver-\(ProcessInfo.processInfo.processIdentifier)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

        let node = try await Iroh.persistent(path: tempDir.path)

        // Wait for node to be online
        try await node.net().waitOnline()

        let myId = node.net().nodeId()

        // Convert channel name to topic ID using blake3
        let topic = topicFromName(channel)

        // Subscribe to the topic with no bootstrap peers
        let callback = ReceiverCallback()
        let sender = try await node.gossip().subscribe(
            topic: topic,
            bootstrap: [],
            cb: callback
        )

        // Keep sender reference alive
        _ = sender

        printFlush("Receiver running")
        printFlush("Endpoint ID: \(myId)")
        printFlush("Listening on channel `\(channel)`...\n")

        // Run forever (until Ctrl+C)
        await withCheckedContinuation { (_: CheckedContinuation<Void, Never>) in
            // Never resume - run until killed
        }
    }
}
