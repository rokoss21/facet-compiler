#!/bin/bash
set -e

echo "🚀 Building FACET v2.0 WebAssembly module..."

# Install wasm-pack if not present
if ! command -v wasm-pack &> /dev/null; then
    echo "📦 Installing wasm-pack..."
    cargo install wasm-pack
fi

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf pkg pkg-node pkg-bundler

# Build for web (browsers)
echo "🌐 Building for web browsers..."
wasm-pack build --target web --out-dir pkg -- --release

# Build for Node.js
echo "📦 Building for Node.js..."
wasm-pack build --target nodejs --out-dir pkg-node -- --release

# Build for bundlers (webpack, rollup, etc.)
echo "📦 Building for bundlers..."
wasm-pack build --target bundler --out-dir pkg-bundler -- --release

# Optimize WASM files
echo "⚡ Optimizing WASM files..."
if command -v wasm-opt &> /dev/null; then
    wasm-opt -Oz --enable-bulk-memory pkg/fct_wasm_bg.wasm -o pkg/fct_wasm_bg.wasm
    wasm-opt -Oz --enable-bulk-memory pkg-node/fct_wasm_bg.wasm -o pkg-node/fct_wasm_bg.wasm
    wasm-opt -Oz --enable-bulk-memory pkg-bundler/fct_wasm_bg.wasm -o pkg-bundler/fct_wasm_bg.wasm
else
    echo "⚠️  wasm-opt not found. Install binaryen for better optimization"
fi

# Create npm package
echo "📋 Creating npm package..."
cp package.json pkg/
cp README.md pkg/
cp LICENSE* pkg/ 2>/dev/null || true

# Show sizes
echo "📊 Build sizes:"
echo "Web: $(du -h pkg/fct_wasm_bg.wasm | cut -f1)"
echo "Node.js: $(du -h pkg-node/fct_wasm_bg.wasm | cut -f1)"
echo "Bundler: $(du -h pkg-bundler/fct_wasm_bg.wasm | cut -f1)"

echo "✅ Build complete!"
echo ""
echo "To test locally:"
echo "  npm pack"
echo "  npm install ./facet-fct-wasm-0.1.0.tgz"
echo ""
echo "To publish to npm:"
echo "  cd pkg"
echo "  npm publish"