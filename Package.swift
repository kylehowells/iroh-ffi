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
            checksum: "ccdbf9d9e2b2ca701ba2e5b94791f63de25f5de2c505301e33c2504ee219f89f"),
        // macOS (Apple Silicon): 60 MB download
        .binaryTarget(
            name: "IrohMacOS",
            url: "https://github.com/kylehowells/iroh-ffi/releases/download/v0.96.0/Iroh-macos.xcframework.zip",
            checksum: "2ce7c5815425b837a4c37d0105a3e4d81548d7e05cba922588036e20eee21faa"),
    ]
)
