/// Swift gossip sender - compatible with GossipDemo
///
/// Usage:
///   sender --peer <endpoint_id> [--channel <name>] <message>
///
/// Sends a gossip message to peers on a channel

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

/// Callback handler for gossip messages (needed to receive join confirmation)
class SenderCallback: GossipMessageCallback {
    var connected = false
    var continuation: CheckedContinuation<Void, Never>?

    func onMessage(msg: Message) async throws {
        switch msg.type() {
        case .neighborUp:
            connected = true
            continuation?.resume()
            continuation = nil
        case .neighborDown:
            break
        case .received:
            // We might receive our own message back or others
            break
        case .lagged:
            break
        case .error:
            let err = msg.asError()
            printFlush("Error: \(err)")
        }
    }

    func waitForConnection() async {
        if connected { return }
        await withCheckedContinuation { cont in
            if connected {
                cont.resume()
            } else {
                continuation = cont
            }
        }
    }
}

@main
struct Sender: AsyncParsableCommand {
    static var configuration = CommandConfiguration(
        commandName: "sender",
        abstract: "Send a gossip message to peers on a topic"
    )

    @Option(name: .long, help: "Endpoint ID of the receiver to connect to")
    var peer: String

    @Option(name: .long, help: "Channel/topic name")
    var channel: String = "chat"

    @Argument(help: "Message to send")
    var message: String

    mutating func run() async throws {
        // Create node with persistent storage in temp directory
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("iroh-sender-\(ProcessInfo.processInfo.processIdentifier)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

        let node = try await Iroh.persistent(path: tempDir.path)

        // Wait for node to be online
        try await node.net().waitOnline()

        // Convert channel name to topic ID using blake3
        let topic = topicFromName(channel)

        printFlush("Connecting to peer \(String(peer.prefix(10)))...")

        // Subscribe to the topic with the receiver as bootstrap peer
        let callback = SenderCallback()
        let sender = try await node.gossip().subscribe(
            topic: topic,
            bootstrap: [peer],
            cb: callback
        )

        // Wait for peer connection
        printFlush("Waiting for peer connection...")

        // Race between connection and timeout
        await withTaskGroup(of: Bool.self) { group in
            group.addTask {
                await callback.waitForConnection()
                return true
            }
            group.addTask {
                try? await Task.sleep(nanoseconds: 30_000_000_000)
                return false
            }

            if let result = await group.next() {
                if !result {
                    printFlush("Timeout waiting for peer connection")
                    group.cancelAll()
                    return
                }
            }
            group.cancelAll()
        }

        if !callback.connected {
            printFlush("Failed to connect to peer")
            try await node.node().shutdown()
            return
        }

        printFlush("Connected!")

        // Send the message
        try await sender.broadcast(msg: Data(message.utf8))
        printFlush("Message sent: \(message)")

        // Give a moment for the message to propagate
        try await Task.sleep(nanoseconds: 500_000_000) // 500ms

        // Shutdown
        try await sender.cancel()
        try await node.node().shutdown()

        printFlush("Done.")
    }
}
