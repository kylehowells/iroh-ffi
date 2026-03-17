// swift-tools-version:5.8
import PackageDescription

let package = Package(
    name: "IrohLib",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    products: [
        .library(
            name: "IrohLib",
            targets: ["IrohLib"]),
        .executable(
            name: "GossipChat",
            targets: ["GossipChat"]),
        .executable(
            name: "sender",
            targets: ["GossipSender"]),
        .executable(
            name: "receiver",
            targets: ["GossipReceiver"]),
        .executable(
            name: "sender-swift5",
            targets: ["GossipSenderSwift5"]),
        .executable(
            name: "BlobDemo",
            targets: ["BlobDemo"]),
        .executable(
            name: "DocDemo",
            targets: ["DocDemo"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser", from: "1.2.0"),
    ],
    targets: [
        .target(
            name: "IrohLib",
            dependencies: [
                .target(name: "IrohiOS", condition: .when(platforms: [.iOS])),
                .target(name: "IrohMacOS", condition: .when(platforms: [.macOS])),
            ],
            linkerSettings: [
              .linkedFramework("SystemConfiguration")
            ]),

        // Per-platform binary targets — SwiftPM only links the one matching your build platform.
        .binaryTarget(
            name: "IrohiOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.97.0/Iroh-ios.xcframework.zip",
            checksum: "69207361cd4837fb7d91d0da50d608016d7af66a0f2e3af726b74f48b319fbaa"),
        .binaryTarget(
            name: "IrohMacOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.97.0/Iroh-macos.xcframework.zip",
            checksum: "eeb3ac2db0b53de576b57faa4c18a21e91d84d475e740faecf2746cc9e51dc8f"),

        // For local development, comment out URL targets above and uncomment:
        // .binaryTarget(name: "IrohiOS", path: "../Iroh-ios.xcframework"),
        // .binaryTarget(name: "IrohMacOS", path: "../Iroh-macos.xcframework"),

        .testTarget(
          name: "IrohLibTests",
          dependencies: ["IrohLib"]),
        .executableTarget(
            name: "GossipChat",
            dependencies: ["IrohLib"],
            path: "Sources/GossipChat",
            linkerSettings: [
              .linkedFramework("SystemConfiguration"),
              .linkedFramework("Security")
            ]),
        .executableTarget(
            name: "GossipSender",
            dependencies: [
                "IrohLib",
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ],
            path: "Sources/GossipSender",
            linkerSettings: [
              .linkedFramework("SystemConfiguration"),
              .linkedFramework("Security")
            ]),
        .executableTarget(
            name: "GossipReceiver",
            dependencies: [
                "IrohLib",
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ],
            path: "Sources/GossipReceiver",
            linkerSettings: [
              .linkedFramework("SystemConfiguration"),
              .linkedFramework("Security")
            ]),
        .executableTarget(
            name: "GossipSenderSwift5",
            dependencies: [
                "IrohLib",
            ],
            path: "Sources/GossipSenderSwift5",
            linkerSettings: [
              .linkedFramework("SystemConfiguration"),
              .linkedFramework("Security")
            ]),
        .executableTarget(
            name: "BlobDemo",
            dependencies: ["IrohLib"],
            path: "Sources/BlobDemo",
            linkerSettings: [
              .linkedFramework("SystemConfiguration"),
              .linkedFramework("Security")
            ]),
        .executableTarget(
            name: "DocDemo",
            dependencies: ["IrohLib"],
            path: "Sources/DocDemo",
            linkerSettings: [
              .linkedFramework("SystemConfiguration"),
              .linkedFramework("Security")
            ]),
    ]
)
