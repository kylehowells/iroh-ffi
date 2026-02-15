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
        // iOS (device + simulator): 177 MB download
        .binaryTarget(
            name: "IrohiOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.96.0/Iroh-ios.xcframework.zip",
            checksum: "193c3c36d0f8127bf6f38292f14d5b18d07683ace992b92eed5bf5809f58957b"),
        // macOS (Apple Silicon): 60 MB download
        .binaryTarget(
            name: "IrohMacOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.96.0/Iroh-macos.xcframework.zip",
            checksum: "15ac8d080ffcdcedb624768dd8c668a03527d6f8ee6832f976c1ab725eb3fde4"),

        // For local development, comment out the URL targets above and uncomment:
        // .binaryTarget(
        //     name: "Iroh",
        //     path: "artifacts/Iroh.xcframework"),
        // Then change IrohLib dependencies to: .byName(name: "Iroh")

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
