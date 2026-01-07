/// Swift document demo using iroh-ffi
///
/// Usage:
///   swift run DocDemo create
///   swift run DocDemo join [TICKET]
///
/// Examples:
///   # Create a document and wait for peers
///   swift run DocDemo create
///
///   # Join an existing document
///   swift run DocDemo join <TICKET>

import Foundation
import IrohLib

class DocCallback: SubscribeCallback {
    let name: String

    init(name: String) {
        self.name = name
    }

    func event(event: LiveEvent) async throws {
        switch event.type() {
        case .insertLocal:
            let entry = event.asInsertLocal()
            let keyData = entry.key()
            let key = String(data: keyData, encoding: .utf8) ?? keyData.hexString
            print("\n[\(name)] Local insert: key='\(key)', content_len=\(entry.contentLen())")
        case .insertRemote:
            let insert = event.asInsertRemote()
            let keyData = insert.entry.key()
            let key = String(data: keyData, encoding: .utf8) ?? keyData.hexString
            print("\n[\(name)] Remote insert from \(insert.from): key='\(key)', content_len=\(insert.entry.contentLen())")
        case .contentReady:
            let hash = event.asContentReady()
            print("\n[\(name)] Content ready: \(hash)")
        case .syncFinished:
            let sync = event.asSyncFinished()
            print("\n[\(name)] Sync finished with peer: \(sync.peer)")
        case .neighborUp:
            let peer = event.asNeighborUp()
            print("\n[\(name)] Neighbor up: \(peer)")
        case .neighborDown:
            let peer = event.asNeighborDown()
            print("\n[\(name)] Neighbor down: \(peer)")
        case .pendingContentReady:
            print("\n[\(name)] Pending content ready")
        }
        print("> ", terminator: "")
        fflush(stdout)
    }
}

extension Data {
    var hexString: String {
        return map { String(format: "%02x", $0) }.joined()
    }
}

func printUsage() {
    print("Usage:")
    print("  swift run DocDemo create")
    print("  swift run DocDemo join [TICKET]")
}

func createAndHostDoc(node: Iroh, author: AuthorId) async throws {
    print("Creating new document...")
    let doc = try await node.docs().create()
    print("Document ID: \(doc.id())")

    // Subscribe to events
    let callback = DocCallback(name: "doc")
    try await doc.subscribe(cb: callback)
    print("Subscribed to document events")

    // Set some initial entries
    print("\nSetting initial entries...")
    _ = try await doc.setBytes(authorId: author, key: "greeting".data(using: .utf8)!, value: "Hello from Swift!".data(using: .utf8)!)
    _ = try await doc.setBytes(authorId: author, key: "count".data(using: .utf8)!, value: "0".data(using: .utf8)!)
    print("Initial entries set")

    // Create and share ticket
    let ticket = try await doc.share(mode: .write, addrOptions: .relayAndAddresses)

    print("\n=== DOC TICKET ===")
    print("\(ticket)")
    print("==================\n")
    print("To join this document, run:")
    print("  swift run DocDemo join \(ticket)")

    print("\n=== Interactive Mode ===")
    print("Commands:")
    print("  set <key> <value>  - Set a key-value pair")
    print("  get <key>          - Get a value by key")
    print("  list               - List all entries")
    print("  /quit              - Exit\n")

    // Interactive loop
    print("> ", terminator: "")
    fflush(stdout)

    while let line = readLine() {
        let parts = line.trimmingCharacters(in: .whitespaces).split(separator: " ", omittingEmptySubsequences: false)

        if parts.isEmpty {
            print("> ", terminator: "")
            fflush(stdout)
            continue
        }

        let cmd = String(parts[0])

        if cmd == "/quit" || cmd == "quit" || cmd == "exit" {
            break
        } else if cmd == "set" && parts.count >= 3 {
            let key = String(parts[1]).data(using: .utf8)!
            let value = parts.dropFirst(2).joined(separator: " ").data(using: .utf8)!
            let hash = try await doc.setBytes(authorId: author, key: key, value: value)
            print("Set '\(parts[1])' = '\(parts.dropFirst(2).joined(separator: " "))' (hash: \(hash))")
        } else if cmd == "get" && parts.count >= 2 {
            let key = String(parts[1]).data(using: .utf8)!
            let query = Query.keyExact(key: key, opts: nil)
            if let entry = try await doc.getOne(query: query) {
                let content = try await node.blobs().readToBytes(hash: entry.contentHash())
                let value = String(data: Data(content), encoding: .utf8) ?? Data(content).hexString
                print("'\(parts[1])' = '\(value)'")
            } else {
                print("Key '\(parts[1])' not found")
            }
        } else if cmd == "list" {
            let query = Query.all(opts: nil)
            let entries = try await doc.getMany(query: query)
            print("Entries (\(entries.count)):")
            for entry in entries {
                let keyData = entry.key()
                let key = String(data: keyData, encoding: .utf8) ?? keyData.hexString
                let content = try await node.blobs().readToBytes(hash: entry.contentHash())
                let value = String(data: Data(content), encoding: .utf8) ?? Data(content).hexString
                print("  '\(key)' = '\(value)'")
            }
        } else {
            print("Unknown command. Try: set, get, list, /quit")
        }

        print("> ", terminator: "")
        fflush(stdout)
    }
}

func joinDoc(node: Iroh, author: AuthorId, ticketStr: String) async throws {
    print("Parsing ticket...")
    let ticket = try DocTicket(str: ticketStr)

    print("\nJoining document...")
    let callback = DocCallback(name: "doc")
    let doc = try await node.docs().joinAndSubscribe(ticket: ticket, cb: callback)
    print("Joined document: \(doc.id())")

    // Give it a moment to sync
    try await Task.sleep(nanoseconds: 2_000_000_000)

    // List existing entries
    print("\nExisting entries:")
    let query = Query.all(opts: nil)
    let entries = try await doc.getMany(query: query)
    for entry in entries {
        let keyData = entry.key()
        let key = String(data: keyData, encoding: .utf8) ?? keyData.hexString
        let content = try await node.blobs().readToBytes(hash: entry.contentHash())
        let value = String(data: Data(content), encoding: .utf8) ?? Data(content).hexString
        print("  '\(key)' = '\(value)'")
    }

    print("\n=== Interactive Mode ===")
    print("Commands:")
    print("  set <key> <value>  - Set a key-value pair")
    print("  get <key>          - Get a value by key")
    print("  list               - List all entries")
    print("  /quit              - Exit\n")

    // Interactive loop
    print("> ", terminator: "")
    fflush(stdout)

    while let line = readLine() {
        let parts = line.trimmingCharacters(in: .whitespaces).split(separator: " ", omittingEmptySubsequences: false)

        if parts.isEmpty {
            print("> ", terminator: "")
            fflush(stdout)
            continue
        }

        let cmd = String(parts[0])

        if cmd == "/quit" || cmd == "quit" || cmd == "exit" {
            break
        } else if cmd == "set" && parts.count >= 3 {
            let key = String(parts[1]).data(using: .utf8)!
            let value = parts.dropFirst(2).joined(separator: " ").data(using: .utf8)!
            let hash = try await doc.setBytes(authorId: author, key: key, value: value)
            print("Set '\(parts[1])' = '\(parts.dropFirst(2).joined(separator: " "))' (hash: \(hash))")
        } else if cmd == "get" && parts.count >= 2 {
            let key = String(parts[1]).data(using: .utf8)!
            let query = Query.keyExact(key: key, opts: nil)
            if let entry = try await doc.getOne(query: query) {
                let content = try await node.blobs().readToBytes(hash: entry.contentHash())
                let value = String(data: Data(content), encoding: .utf8) ?? Data(content).hexString
                print("'\(parts[1])' = '\(value)'")
            } else {
                print("Key '\(parts[1])' not found")
            }
        } else if cmd == "list" {
            let query = Query.all(opts: nil)
            let entries = try await doc.getMany(query: query)
            print("Entries (\(entries.count)):")
            for entry in entries {
                let keyData = entry.key()
                let key = String(data: keyData, encoding: .utf8) ?? keyData.hexString
                let content = try await node.blobs().readToBytes(hash: entry.contentHash())
                let value = String(data: Data(content), encoding: .utf8) ?? Data(content).hexString
                print("  '\(key)' = '\(value)'")
            }
        } else {
            print("Unknown command. Try: set, get, list, /quit")
        }

        print("> ", terminator: "")
        fflush(stdout)
    }
}

@main
struct DocDemo {
    static func main() async throws {
        // Force unbuffered output for terminal visibility
        setbuf(stdout, nil)
        setbuf(stderr, nil)

        let args = Array(CommandLine.arguments.dropFirst())

        if args.isEmpty {
            printUsage()
            return
        }

        let command = args[0]

        print("=== Iroh Doc Demo (Swift) ===\n")
        fflush(stdout)

        // Create node with docs enabled in temp directory
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("iroh-doc-swift-\(ProcessInfo.processInfo.processIdentifier)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

        print("Creating node at \(tempDir.path)...")
        fflush(stdout)

        let options = NodeOptions(
            gcIntervalMillis: nil,
            blobEvents: nil,
            enableDocs: true,
            ipv4Addr: nil,
            ipv6Addr: nil,
            nodeDiscovery: nil,
            secretKey: nil,
            protocols: nil
        )
        let node = try await Iroh.persistentWithOptions(path: tempDir.path, options: options)

        // Wait for node to be online
        print("Waiting for node to come online...")
        fflush(stdout)
        try await node.net().waitOnline()

        let myId = node.net().nodeId()
        let myAddr = node.net().nodeAddr()
        print("Node ID: \(myId)")
        print("Relay URL: \(myAddr.relayUrl() ?? "none")\n")
        fflush(stdout)

        // Get or create an author
        let authors = try await node.authors().list()
        let author: AuthorId
        if let existingAuthor = authors.first {
            print("Using existing author: \(existingAuthor)")
            author = existingAuthor
        } else {
            let newAuthor = try await node.authors().create()
            print("Created new author: \(newAuthor)")
            author = newAuthor
        }
        fflush(stdout)

        switch command {
        case "create":
            try await createAndHostDoc(node: node, author: author)
        case "join":
            guard args.count >= 2 else {
                print("Usage: swift run DocDemo join [TICKET]")
                return
            }
            let ticketStr = args[1]
            try await joinDoc(node: node, author: author, ticketStr: ticketStr)
        default:
            printUsage()
        }

        print("\nShutting down...")
        try await node.node().shutdown()
    }
}
