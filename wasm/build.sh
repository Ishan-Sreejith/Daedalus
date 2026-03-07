#!/bin/bash

# CoRe Language WebAssembly Build Script
set -e

echo "🌟 Building CoRe Language for WebAssembly..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found!"
    echo "📦 Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    echo "✅ wasm-pack installed!"
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Cargo.toml not found. Run this script from the wasm/ directory."
    exit 1
fi

echo "🔨 Building WebAssembly module..."

# Build the WebAssembly package
wasm-pack build --target web --out-dir pkg

if [ $? -eq 0 ]; then
    echo "✅ WebAssembly build completed successfully!"
    echo ""
    echo "📁 Generated files:"
    ls -la pkg/ | head -10
    echo ""
    echo "🌐 To serve the web IDE:"
    echo "   cd $(pwd)"
    echo "   python3 -m http.server 8000"
    echo "   # OR"
    echo "   npx serve ."
    echo ""
    echo "   Then open: http://localhost:8000"
    echo ""
    echo "🚀 Happy coding with CoRe Language!"
else
    echo "❌ Build failed!"
    exit 1
fi
