#!/bin/bash

# FACET v2.0 Release Build Script
# Builds optimized binaries for distribution

set -e

echo "🚀 Building FACET v2.0 Release Binaries"
echo "========================================"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Build release binary
echo "🔨 Building release binary..."
cargo build --release

# Verify binary works
echo "✅ Testing binary..."
./target/release/facet-fct --version
./target/release/facet-fct build --input examples/basic.facet

# Get binary size
if [[ "$OSTYPE" == "darwin"* ]]; then
    SIZE=$(stat -f%z ./target/release/facet-fct)
else
    SIZE=$(stat -c%s ./target/release/facet-fct)
fi

echo "📦 Binary size: $SIZE bytes"
echo ""
echo "🎉 Release build complete!"
echo "Binary location: ./target/release/facet-fct"
echo ""
echo "For distribution:"
echo "- Linux: tar czf facet-fct-linux-x86_64.tar.gz ./target/release/facet-fct"
echo "- macOS: tar czf facet-fct-macos-x86_64.tar.gz ./target/release/facet-fct"
echo "- Windows: zip facet-fct-windows-x86_64.zip ./target/release/facet-fct.exe"


# FACET v2.0 Release Build Script
# Builds optimized binaries for distribution

set -e

echo "🚀 Building FACET v2.0 Release Binaries"
echo "========================================"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Build release binary
echo "🔨 Building release binary..."
cargo build --release

# Verify binary works
echo "✅ Testing binary..."
./target/release/facet-fct --version
./target/release/facet-fct build --input examples/basic.facet

# Get binary size
if [[ "$OSTYPE" == "darwin"* ]]; then
    SIZE=$(stat -f%z ./target/release/facet-fct)
else
    SIZE=$(stat -c%s ./target/release/facet-fct)
fi

echo "📦 Binary size: $SIZE bytes"
echo ""
echo "🎉 Release build complete!"
echo "Binary location: ./target/release/facet-fct"
echo ""
echo "For distribution:"
echo "- Linux: tar czf facet-fct-linux-x86_64.tar.gz ./target/release/facet-fct"
echo "- macOS: tar czf facet-fct-macos-x86_64.tar.gz ./target/release/facet-fct"
echo "- Windows: zip facet-fct-windows-x86_64.zip ./target/release/facet-fct.exe"


