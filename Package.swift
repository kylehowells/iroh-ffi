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
    ],
    dependencies: [],
    targets: [
        .target(
            name: "IrohLib",
            dependencies: [
                .target(name: "IrohiOS", condition: .when(platforms: [.iOS])),
                .target(name: "IrohMacOS", condition: .when(platforms: [.macOS])),
            ],
            path: "IrohLib/Sources/IrohLib",
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
    ]
)
