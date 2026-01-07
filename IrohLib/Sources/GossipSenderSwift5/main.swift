/// Swift 5 gossip sender - using completion handlers and delegates (no async/await)
///
/// Usage:
///   sender-swift5 <peer_id> [message]
///
/// Demonstrates the completion-handler/delegate pattern for Swift 5 compatibility

import Foundation
import IrohLib

// MARK: - Completion Handler Extensions

extension Iroh {
    /// Completion-handler version of persistent()
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

extension Net {
    /// Completion-handler version of waitOnline()
    func waitOnline(
        queue: DispatchQueue = .main,
        completion: @escaping (Result<Void, Error>) -> Void
    ) {
        Task {
            do {
                try await self.waitOnline()
                queue.async { completion(.success(())) }
            } catch {
                queue.async { completion(.failure(error)) }
            }
        }
    }
}

extension Gossip {
    /// Completion-handler version of subscribe()
    func subscribe(
        topic: Data,
        bootstrap: [String],
        delegate: GossipDelegate,
        queue: DispatchQueue = .main,
        completion: @escaping (Result<Sender, Error>) -> Void
    ) {
        let callback = DelegateCallbackAdapter(delegate: delegate, queue: queue)
        Task {
            do {
                let sender = try await self.subscribe(topic: topic, bootstrap: bootstrap, cb: callback)
                queue.async { completion(.success(sender)) }
            } catch {
                queue.async { completion(.failure(error)) }
            }
        }
    }
}

extension Sender {
    /// Completion-handler version of broadcast()
    func broadcast(
        msg: Data,
        queue: DispatchQueue = .main,
        completion: @escaping (Result<Void, Error>) -> Void
    ) {
        Task {
            do {
                try await self.broadcast(msg: msg)
                queue.async { completion(.success(())) }
            } catch {
                queue.async { completion(.failure(error)) }
            }
        }
    }

    /// Completion-handler version of cancel()
    func cancel(
        queue: DispatchQueue = .main,
        completion: @escaping (Result<Void, Error>) -> Void
    ) {
        Task {
            do {
                try await self.cancel()
                queue.async { completion(.success(())) }
            } catch {
                queue.async { completion(.failure(error)) }
            }
        }
    }
}

extension Node {
    /// Completion-handler version of shutdown()
    func shutdown(
        queue: DispatchQueue = .main,
        completion: @escaping (Result<Void, Error>) -> Void
    ) {
        Task {
            do {
                try await self.shutdown()
                queue.async { completion(.success(())) }
            } catch {
                queue.async { completion(.failure(error)) }
            }
        }
    }
}

// MARK: - Delegate Protocol

protocol GossipDelegate: AnyObject {
    func gossipDidReceiveMessage(_ content: Data, from peer: String)
    func gossipPeerDidConnect(_ peer: String)
    func gossipPeerDidDisconnect(_ peer: String)
    func gossipDidEncounterError(_ error: String)
    func gossipDidLag()
}

// Default implementations
extension GossipDelegate {
    func gossipDidLag() {}
}

// MARK: - Delegate Adapter

/// Bridges the delegate pattern to the async GossipMessageCallback
class DelegateCallbackAdapter: GossipMessageCallback {
    weak var delegate: GossipDelegate?
    let queue: DispatchQueue

    init(delegate: GossipDelegate, queue: DispatchQueue) {
        self.delegate = delegate
        self.queue = queue
    }

    func onMessage(msg: Message) async throws {
        // Capture values before switching to queue
        let msgType = msg.type()

        queue.async { [weak self] in
            guard let delegate = self?.delegate else { return }

            switch msgType {
            case .received:
                let received = msg.asReceived()
                let content = Data(received.content)
                let from = received.deliveredFrom
                delegate.gossipDidReceiveMessage(content, from: from)
            case .neighborUp:
                let peer = msg.asNeighborUp()
                delegate.gossipPeerDidConnect(peer)
            case .neighborDown:
                let peer = msg.asNeighborDown()
                delegate.gossipPeerDidDisconnect(peer)
            case .error:
                let error = msg.asError()
                delegate.gossipDidEncounterError(error)
            case .lagged:
                delegate.gossipDidLag()
            }
        }
    }
}

// MARK: - Topic Helper

/// Pre-computed blake3 hash for "chat" channel
/// blake3("chat") = 504c1dbb87fc1cd93594bd6baad1b520229bd222e16d9c48138998f602993c67
func chatTopic() -> Data {
    return Data(hexString: "504c1dbb87fc1cd93594bd6baad1b520229bd222e16d9c48138998f602993c67")!
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

// MARK: - GossipSender Class

/// Main sender class using delegate pattern
class GossipSender: GossipDelegate {
    private var node: Iroh?
    private var sender: Sender?
    private let peerID: String
    private let message: String
    private let semaphore = DispatchSemaphore(value: 0)
    private var connected = false
    private var error: Error?

    init(peerID: String, message: String) {
        self.peerID = peerID
        self.message = message
    }

    /// Run the sender (blocking)
    func run() -> Bool {
        print("Starting node...")
        fflush(stdout)

        // Step 1: Create node
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("iroh-sender5-\(ProcessInfo.processInfo.processIdentifier)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

        Iroh.persistent(path: tempDir.path, queue: .main) { [weak self] result in
            switch result {
            case .success(let node):
                self?.node = node
                self?.waitForOnline()
            case .failure(let error):
                print("Failed to create node: \(error)")
                fflush(stdout)
                self?.error = error
                self?.semaphore.signal()
            }
        }

        // Wait for completion
        semaphore.wait()

        return error == nil && connected
    }

    private func waitForOnline() {
        print("Waiting for node to come online...")
        fflush(stdout)

        node?.net().waitOnline(queue: .main) { [weak self] result in
            switch result {
            case .success:
                self?.subscribeToTopic()
            case .failure(let error):
                print("Failed to come online: \(error)")
                fflush(stdout)
                self?.error = error
                self?.semaphore.signal()
            }
        }
    }

    private func subscribeToTopic() {
        print("Connecting to peer \(String(peerID.prefix(10)))...")
        fflush(stdout)

        node?.gossip().subscribe(
            topic: chatTopic(),
            bootstrap: [peerID],
            delegate: self,
            queue: .main
        ) { [weak self] result in
            switch result {
            case .success(let sender):
                self?.sender = sender
                print("Subscribed, waiting for peer connection...")
                fflush(stdout)
                // Now we wait for gossipPeerDidConnect via delegate

                // Set a timeout
                DispatchQueue.main.asyncAfter(deadline: .now() + 30) {
                    if self?.connected == false {
                        print("Timeout waiting for peer connection")
                        fflush(stdout)
                        self?.semaphore.signal()
                    }
                }
            case .failure(let error):
                print("Failed to subscribe: \(error)")
                fflush(stdout)
                self?.error = error
                self?.semaphore.signal()
            }
        }
    }

    private func sendMessage() {
        print("Connected! Sending message...")
        fflush(stdout)

        sender?.broadcast(msg: Data(message.utf8), queue: .main) { [weak self] result in
            switch result {
            case .success:
                print("Message sent: \(self?.message ?? "")")
                fflush(stdout)
                self?.cleanup()
            case .failure(let error):
                print("Failed to send: \(error)")
                fflush(stdout)
                self?.error = error
                self?.cleanup()
            }
        }
    }

    private func cleanup() {
        // Give message time to propagate
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { [weak self] in
            self?.sender?.cancel(queue: .main) { _ in
                self?.node?.node().shutdown(queue: .main) { _ in
                    print("Done.")
                    fflush(stdout)
                    self?.semaphore.signal()
                }
            }
        }
    }

    // MARK: - GossipDelegate

    func gossipPeerDidConnect(_ peer: String) {
        print(":: Peer connected: \(String(peer.prefix(10)))")
        fflush(stdout)
        connected = true
        sendMessage()
    }

    func gossipPeerDidDisconnect(_ peer: String) {
        print(":: Peer disconnected: \(String(peer.prefix(10)))")
        fflush(stdout)
    }

    func gossipDidReceiveMessage(_ content: Data, from peer: String) {
        let text = String(data: content, encoding: .utf8) ?? "<binary>"
        print("[\(String(peer.prefix(10)))] > \(text)")
        fflush(stdout)
    }

    func gossipDidEncounterError(_ error: String) {
        print(":: Error: \(error)")
        fflush(stdout)
    }
}

// MARK: - Main

func printUsage() {
    print("""
        USAGE: sender-swift5 <peer_id> [message]

        ARGUMENTS:
          <peer_id>    Endpoint ID of the receiver to connect to
          [message]    Message to send (default: "Hello from Swift 5!")

        EXAMPLE:
          sender-swift5 abc123def456... "Hello World"
        """)
}

// Parse arguments
let args = CommandLine.arguments
guard args.count >= 2 else {
    printUsage()
    exit(1)
}

let peerID = args[1]
let message = args.count >= 3 ? args[2] : "Hello from Swift 5!"

// Validate peer ID (should be 64 hex chars)
guard peerID.count == 64, peerID.allSatisfy({ $0.isHexDigit }) else {
    print("Error: Invalid peer ID. Expected 64 hex characters.")
    exit(1)
}

// Run on main queue (required for GCD callbacks)
let sender = GossipSender(peerID: peerID, message: message)

// Run the main loop on a background thread so main thread can process callbacks
DispatchQueue.global().async {
    let success = sender.run()
    exit(success ? 0 : 1)
}

// Keep the main run loop alive for callbacks
RunLoop.main.run()
