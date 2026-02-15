set -eu

# TODO: convert to rust

# Env
UDL_NAME="iroh_ffi"
FRAMEWORK_NAME="Iroh"
SWIFT_INTERFACE="IrohLib"
INCLUDE_DIR="include/apple"
HEADERS_DIR="include/xcframework-headers"

# Build default lib
cargo build --lib

# Compile the rust
echo "Building aarch64-apple-ios"
cargo build --release --target aarch64-apple-ios
echo "Building aarch64-apple-ios-sim"
cargo build --release --target aarch64-apple-ios-sim
echo "Building x86_64-apple-ios"
cargo build --release --target x86_64-apple-ios
echo "Building aarch64-apple-darwin"
cargo build --release --target aarch64-apple-darwin

rm -f ./target/universal.a
rm -f $INCLUDE_DIR/*

# Make dirs if they don't exist
mkdir -p $INCLUDE_DIR
mkdir -p $HEADERS_DIR

# UniFfi bindgen
cargo run --bin uniffi-bindgen generate --language swift --out-dir ./$INCLUDE_DIR --library target/debug/libiroh_ffi.dylib --config uniffi.toml

# Make fat lib for sims
lipo -create \
    "./target/aarch64-apple-ios-sim/release/lib${UDL_NAME}.a" \
    "./target/x86_64-apple-ios/release/lib${UDL_NAME}.a" \
    -output ./target/universal.a

# Prepare headers for library-based xcframework
cp "$INCLUDE_DIR/${UDL_NAME}FFI.h" "$HEADERS_DIR/${UDL_NAME}FFI.h"
cat > "$HEADERS_DIR/module.modulemap" << EOF
module ${FRAMEWORK_NAME} {
  header "${UDL_NAME}FFI.h"
  export *
}
EOF

# Build library-based xcframeworks using xcodebuild
echo "Creating xcframeworks..."

rm -rf "${FRAMEWORK_NAME}-ios.xcframework"
xcodebuild -create-xcframework \
    -library "./target/aarch64-apple-ios/release/lib${UDL_NAME}.a" \
    -headers "$HEADERS_DIR" \
    -library ./target/universal.a \
    -headers "$HEADERS_DIR" \
    -output "${FRAMEWORK_NAME}-ios.xcframework"

rm -rf "${FRAMEWORK_NAME}-macos.xcframework"
xcodebuild -create-xcframework \
    -library "./target/aarch64-apple-darwin/release/lib${UDL_NAME}.a" \
    -headers "$HEADERS_DIR" \
    -output "${FRAMEWORK_NAME}-macos.xcframework"

# Move swift interface
sed "s/${UDL_NAME}FFI/$FRAMEWORK_NAME/g" "$INCLUDE_DIR/$UDL_NAME.swift" > "$INCLUDE_DIR/$SWIFT_INTERFACE.swift"

rm -f "$SWIFT_INTERFACE/Sources/$SWIFT_INTERFACE/$SWIFT_INTERFACE.swift"
cp "$INCLUDE_DIR/$SWIFT_INTERFACE.swift" \
    "$SWIFT_INTERFACE/Sources/$SWIFT_INTERFACE/$SWIFT_INTERFACE.swift"

echo ""
echo "=== Build complete ==="
echo "iOS xcframework:   ${FRAMEWORK_NAME}-ios.xcframework"
echo "macOS xcframework: ${FRAMEWORK_NAME}-macos.xcframework"
echo ""
echo "To create release zips:"
echo "  zip -r ${FRAMEWORK_NAME}-ios.xcframework.zip ${FRAMEWORK_NAME}-ios.xcframework/"
echo "  zip -r ${FRAMEWORK_NAME}-macos.xcframework.zip ${FRAMEWORK_NAME}-macos.xcframework/"
