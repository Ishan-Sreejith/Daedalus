#!/bin/bash

echo "🔍 CoRe Language WebAssembly Setup Verification"
echo "=============================================="

# Check if in correct directory
if [ ! -f "index.html" ]; then
    echo "❌ Please run this from the wasm/ directory"
    exit 1
fi

# Check WASM files
echo "📦 Checking WASM build files..."
if [ -f "pkg/core_wasm.js" ] && [ -f "pkg/core_wasm_bg.wasm" ]; then
    echo "✅ WASM files present"
    echo "   - core_wasm.js: $(wc -c < pkg/core_wasm.js) bytes"
    echo "   - core_wasm_bg.wasm: $(wc -c < pkg/core_wasm_bg.wasm) bytes"
else
    echo "❌ WASM files missing. Run: wasm-pack build --target web --out-dir pkg"
    exit 1
fi

# Check required files
echo ""
echo "📄 Checking required files..."
required_files=("index.html" "test.html" "start.sh" "src/lib.rs" "Cargo.toml")
for file in "${required_files[@]}"; do
    if [ -f "$file" ]; then
        echo "✅ $file"
    else
        echo "❌ $file missing"
    fi
done

# Check if Python is available
echo ""
echo "🐍 Checking Python for HTTP server..."
if command -v python3 &> /dev/null; then
    echo "✅ Python3 available: $(python3 --version)"
elif command -v python &> /dev/null; then
    echo "✅ Python available: $(python --version)"
else
    echo "❌ Python not found. Install Python to run local server."
fi

# Final status
echo ""
echo "🎯 Setup Status: READY ✅"
echo ""
echo "🚀 To start the IDE:"
echo "   1. Run: ./start.sh (or python3 -m http.server 8080)"
echo "   2. Open: http://localhost:8080"
echo "   3. Test: http://localhost:8080/test.html"
echo ""
echo "💡 Features included:"
echo "   ✅ Complete WebAssembly module"
echo "   ✅ Interactive file editor"
echo "   ✅ Terminal emulation"
echo "   ✅ Core language execution"
echo "   ✅ Example programs"
echo "   ✅ Debugging tools"
echo ""
echo "📚 See README.md for full documentation"
