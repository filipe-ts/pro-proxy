#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo "  Building pro-proxy Universal macOS Binary"
echo "  Target OS: macOS 15 Sequoia (Apple Silicon + Intel)"
echo "============================================================"

# Ensure targets are installed
echo "Checking rustup targets..."
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin

# Clean dist
mkdir -p dist

echo "Building release for Apple Silicon (aarch64-apple-darwin)..."
cargo build --release --target aarch64-apple-darwin

echo "Building release for Intel Mac (x86_64-apple-darwin)..."
cargo build --release --target x86_64-apple-darwin

ARM64_BIN="target/aarch64-apple-darwin/release/pro-proxy"
X86_64_BIN="target/x86_64-apple-darwin/release/pro-proxy"
UNIVERSAL_BIN="dist/pro-proxy"

echo "Creating Universal macOS Binary via lipo..."
lipo -create -output "$UNIVERSAL_BIN" "$ARM64_BIN" "$X86_64_BIN"
chmod +x "$UNIVERSAL_BIN"

# Also copy single-arch binaries
cp "$ARM64_BIN" "dist/pro-proxy-arm64"
cp "$X86_64_BIN" "dist/pro-proxy-x86_64"

echo ""
echo "============================================================"
echo "  Build Completed Successfully!"
echo "============================================================"
echo "Universal Binary: $UNIVERSAL_BIN"
lipo -info "$UNIVERSAL_BIN"
file "$UNIVERSAL_BIN"
ls -lh "$UNIVERSAL_BIN"
echo "============================================================"
