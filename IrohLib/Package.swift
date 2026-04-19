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
    dependencies: [
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
        .binaryTarget(name: "IrohiOS", path: "../Iroh-ios.xcframework"),
        .binaryTarget(name: "IrohMacOS", path: "../Iroh-macos.xcframework"),

        .testTarget(
          name: "IrohLibTests",
          dependencies: ["IrohLib"]),
    ]
)
