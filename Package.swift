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
        .binaryTarget(
            name: "IrohiOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.98.1/Iroh-ios.xcframework.zip",
            checksum: "ebde1d225e9b2e0795a5ed3136d9850315b4b7f0c4c1cd236b1e80be3ee566e8"),
        .binaryTarget(
            name: "IrohMacOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.98.1/Iroh-macos.xcframework.zip",
            checksum: "b174cc503d0313444aee11118575f27de02cf6514c6ef5368ff01ebf0be086cf"),
    ]
)
