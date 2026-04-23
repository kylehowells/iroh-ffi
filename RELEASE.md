# Release Process

Each release process is broken down per language target:

## Swift

1. Build the library-based xcframeworks:

```bash
./make_swift.sh
```

This compiles for all Apple platforms (iOS device, iOS simulator, macOS) and produces:
- `Iroh-ios.xcframework/` (device + simulator fat lib)
- `Iroh-macos.xcframework/` (Apple Silicon)

2. Zip and compute checksums:

```bash
zip -r Iroh-ios.xcframework.zip Iroh-ios.xcframework/
zip -r Iroh-macos.xcframework.zip Iroh-macos.xcframework/
swift package compute-checksum Iroh-ios.xcframework.zip
swift package compute-checksum Iroh-macos.xcframework.zip
```

3. Create GitHub release (e.g., `v0.98.1`) and upload both zips as release assets.

4. Update checksums in `Package.swift` (root). `IrohLib/Package.swift` uses local path-based xcframeworks for development:

```swift
.binaryTarget(
    name: "IrohiOS",
    url: "https://github.com/kylehowells/iroh-ffi/releases/download/vX.Y.Z/Iroh-ios.xcframework.zip",
    checksum: "IOS_CHECKSUM"),
.binaryTarget(
    name: "IrohMacOS",
    url: "https://github.com/kylehowells/iroh-ffi/releases/download/vX.Y.Z/Iroh-macos.xcframework.zip",
    checksum: "MACOS_CHECKSUM"),
```

5. Commit, tag, and push:

```bash
git add Package.swift IrohLib/Package.swift
git commit -m "Release vX.Y.Z"
git tag vX.Y.Z
git push origin main --tags
```

Consumers add the package via SwiftPM:

```swift
dependencies: [
    .package(url: "https://github.com/kylehowells/iroh-ffi", from: "0.98.1")
]
```

## Python

The first time:

1) Create an account on [pypi](https://pypi.org/) & [testpipy](https://test.pypi.org/project/iroh/)
2) Get invited to the `iroh` project
3) Install `twine`
4) Upgrade `pkginfo` to at least `1.10`. For more information check out [this issue on twine](https://github.com/pypa/twine/issues/1070)
5) Create an API token on pipy and test pipy
6) Put those tokens into ~/.pypirc:
```
# ~/.pypirc
[pypi]
username = __token__
password = pypi-TOKEN

[testpypi]
username = __token__
password = pypi-TOKEN
```

To release iroh python:

1) Build wheels: `maturin build --release`
2) Upload to testpypi: `twine upload --repository testpypi target/wheels/iroh-$VERSION-*.whl`
3) Test: `pip install -i https://test.pypi.org/simple/ iroh`
4) Upload to pypi: `twine upload target/wheels/iroh-$VERSION-*.whl`
