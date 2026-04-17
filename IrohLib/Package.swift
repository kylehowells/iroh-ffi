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
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.97.1/Iroh-ios.xcframework.zip",
            checksum: "593eda5f6d697f62fc8f9c00fcebde654baed85210f246f067e03b713cdfe904"),
        .binaryTarget(
            name: "IrohMacOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.97.1/Iroh-macos.xcframework.zip",
            checksum: "94f9df44a49d24ea0ccdfa6ef7c80b8677c104b78e5fb0a30006a8e4d83ebc09"),

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
