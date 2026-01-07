/// Swift blob demo using iroh-ffi
///
/// Usage:
///   swift run BlobDemo send [FILE]
///   swift run BlobDemo receive [TICKET] [DEST_FILE]
///   swift run BlobDemo send-bytes [TEXT]
///
/// Examples:
///   # Send a file
///   swift run BlobDemo send ./myfile.txt
///
///   # Receive a file using a ticket
///   swift run BlobDemo receive <TICKET> ./downloaded.txt
///
///   # Send some text as bytes
///   swift run BlobDemo send-bytes "Hello, iroh!"

import Foundation
import IrohLib

class PrintAddCallback: AddCallback {
    let name: String
    var finalHash: Hash? = nil
    var finalFormat: BlobFormat? = nil

    init(name: String) {
        self.name = name
    }

    func progress(progress: AddProgress) async throws {
        switch progress.type() {
        case .found:
            let found = progress.asFound()
            print("[\(name)] Found: \(found.name) (\(found.size) bytes)")
        case .progress:
            let p = progress.asProgress()
            print("\r[\(name)] Progress: \(p.offset) bytes", terminator: "")
            fflush(stdout)
        case .done:
            let done = progress.asDone()
            print("\n[\(name)] Done: \(done.hash)")
        case .allDone:
            let allDone = progress.asAllDone()
            finalHash = allDone.hash
            finalFormat = allDone.format
            print("[\(name)] All done! Hash: \(allDone.hash), Format: \(allDone.format)")
        case .abort:
            let abort = progress.asAbort()
            print("[\(name)] Aborted: \(abort.error)")
        }
    }
}

class PrintDownloadCallback: DownloadCallback {
    let name: String

    init(name: String) {
        self.name = name
    }

    func progress(progress: DownloadProgress) async throws {
        switch progress.type() {
        case .connected:
            print("[\(name)] Connected to peer")
        case .found:
            let found = progress.asFound()
            print("[\(name)] Found blob: \(found.hash) (\(found.size) bytes)")
        case .progress:
            let p = progress.asProgress()
            print("\r[\(name)] Download progress: \(p.offset) bytes", terminator: "")
            fflush(stdout)
        case .done:
            print("\n[\(name)] Blob download complete")
        case .allDone:
            let allDone = progress.asAllDone()
            print("[\(name)] All done! \(allDone.bytesWritten) bytes written, \(allDone.bytesRead) bytes read")
        case .abort:
            let abort = progress.asAbort()
            print("[\(name)] Download aborted: \(abort.error)")
        default:
            break
        }
    }
}

func printUsage() {
    print("Usage:")
    print("  swift run BlobDemo send [FILE]")
    print("  swift run BlobDemo receive [TICKET] [DEST_FILE]")
    print("  swift run BlobDemo send-bytes [TEXT]")
}

func sendFile(node: Iroh, filePath: String) async throws {
    print("Adding file: \(filePath)")

    let absPath = URL(fileURLWithPath: filePath).standardizedFileURL.path
    let callback = PrintAddCallback(name: "add")

    try await node.blobs().addFromPath(
        path: absPath,
        inPlace: false,  // copy, not in-place
        tag: SetTagOption.auto(),
        wrap: WrapOption.noWrap(),
        cb: callback
    )

    // After adding, list blobs to get the hash
    let blobs = try await node.blobs().list()
    if let hashObj = blobs.first {
        // Create a ticket for sharing
        let ticket = try await node.blobs().share(
            hash: hashObj,
            blobFormat: .raw,
            ticketOptions: .id
        )

        print("\n=== BLOB TICKET ===")
        print("\(ticket)")
        print("===================\n")
        print("To download this file, run:")
        print("  swift run BlobDemo receive \(ticket) [DEST_FILE]")
    }
}

func sendBytes(node: Iroh, data: Data) async throws {
    print("Adding \(data.count) bytes of data...")

    let outcome = try await node.blobs().addBytes(bytes: [UInt8](data))

    print("Added blob with hash: \(outcome.hash)")
    print("Format: \(outcome.format)")
    print("Size: \(outcome.size) bytes")

    // Create a ticket for sharing
    let ticket = try await node.blobs().share(
        hash: outcome.hash,
        blobFormat: .raw,
        ticketOptions: .id
    )

    print("\n=== BLOB TICKET ===")
    print("\(ticket)")
    print("===================\n")
    print("To download this blob, run:")
    print("  swift run BlobDemo receive \(ticket) [DEST_FILE]")

    // Also show the content for text data
    if let text = String(data: data, encoding: .utf8) {
        print("\nBlob content: \"\(text)\"")
    }
}

func receiveBlob(node: Iroh, ticketStr: String, destPath: String) async throws {
    print("Parsing ticket...")
    let ticket = try BlobTicket(str: ticketStr)

    let hashObj = ticket.hash()
    let addr = ticket.nodeAddr()

    print("Blob hash: \(hashObj)")
    print("Provider relay URL: \(addr.relayUrl() ?? "none")")

    // Add the node address to discovery for direct connection
    try node.net().addNodeAddr(nodeAddr: addr)

    print("\nDownloading blob...")

    let downloadOpts = try BlobDownloadOptions(
        format: .raw,
        nodes: [addr],
        tag: SetTagOption.auto()
    )

    let callback = PrintDownloadCallback(name: "download")

    try await node.blobs().download(
        hash: hashObj,
        opts: downloadOpts,
        cb: callback
    )

    print("\nExporting to file: \(destPath)")
    try await node.blobs().writeToPath(hash: hashObj, path: destPath)

    print("File saved to: \(destPath)")

    // If it's a small blob, show the content
    let size = try await node.blobs().size(hash: hashObj)
    if size < 1024 {
        let data = try await node.blobs().readToBytes(hash: hashObj)
        if let text = String(data: Data(data), encoding: .utf8) {
            print("\nBlob content: \"\(text)\"")
        }
    }
}

@main
struct BlobDemo {
    static func main() async throws {
        let args = Array(CommandLine.arguments.dropFirst())

        if args.isEmpty {
            printUsage()
            return
        }

        let command = args[0]

        print("=== Iroh Blob Demo (Swift) ===\n")

        // Create node with persistent storage in temp directory
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("iroh-blob-swift-\(ProcessInfo.processInfo.processIdentifier)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

        print("Creating node at \(tempDir.path)...")
        let node = try await Iroh.persistent(path: tempDir.path)

        // Wait for node to be online
        print("Waiting for node to come online...")
        try await node.net().waitOnline()

        let myId = node.net().nodeId()
        let myAddr = node.net().nodeAddr()
        print("Node ID: \(myId)")
        print("Relay URL: \(myAddr.relayUrl() ?? "none")\n")

        switch command {
        case "send":
            guard args.count >= 2 else {
                print("Usage: swift run BlobDemo send [FILE]")
                return
            }
            let filePath = args[1]
            try await sendFile(node: node, filePath: filePath)

            print("\nWaiting for peer to download... Press Ctrl+C to exit.")
            // Keep running
            dispatchMain()

        case "send-bytes":
            guard args.count >= 2 else {
                print("Usage: swift run BlobDemo send-bytes [TEXT]")
                return
            }
            let text = args.dropFirst().joined(separator: " ")
            try await sendBytes(node: node, data: text.data(using: .utf8)!)

            print("\nWaiting for peer to download... Press Ctrl+C to exit.")
            // Keep running
            dispatchMain()

        case "receive":
            guard args.count >= 3 else {
                print("Usage: swift run BlobDemo receive [TICKET] [DEST_FILE]")
                return
            }
            let ticketStr = args[1]
            let destPath = args[2]
            try await receiveBlob(node: node, ticketStr: ticketStr, destPath: destPath)

        default:
            printUsage()
        }

        print("\nShutting down...")
        try await node.node().shutdown()
    }
}
