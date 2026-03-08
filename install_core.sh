#!/bin/bash


CORE_DIR="/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
INSTALL_DIR="$HOME/.local/bin"
RELEASE_DIR="$CORE_DIR/target/release"

echo "🔧 Installing CoRe Language Executables..."

mkdir -p "$INSTALL_DIR"

echo "📦 Copying executables to $INSTALL_DIR..."

if [ -f "$RELEASE_DIR/forge" ]; then
    cp "$RELEASE_DIR/forge" "$INSTALL_DIR/forge"
    echo "✅ Installed forge"
else
    echo "❌ forge not found"
fi

if [ -f "$RELEASE_DIR/fforge" ]; then
    cp "$RELEASE_DIR/fforge" "$INSTALL_DIR/fforge"
    echo "✅ Installed fforge (JIT compiler)"
else
    echo "❌ fforge not found"
fi

if [ -f "$RELEASE_DIR/forger" ]; then
    cp "$RELEASE_DIR/forger" "$INSTALL_DIR/forger"
    echo "✅ Installed forger"
else
    echo "❌ forger not found"
fi

if [ -f "$RELEASE_DIR/metroman" ]; then
    cp "$RELEASE_DIR/metroman" "$INSTALL_DIR/metroman"
    echo "✅ Installed metroman"
else
    echo "❌ metroman not found"
fi

if [ -f "$RELEASE_DIR/test_parser" ]; then
    cp "$RELEASE_DIR/test_parser" "$INSTALL_DIR/test_parser"
    echo "✅ Installed test_parser"
else
    echo "❌ test_parser not found"
fi

chmod +x "$INSTALL_DIR/forge" "$INSTALL_DIR/fforge" "$INSTALL_DIR/forger" "$INSTALL_DIR/metroman" "$INSTALL_DIR/test_parser" 2>/dev/null

echo ""
echo "🎉 CoRe Language Installation Complete!"
echo ""
echo "Available commands:"
echo "  forge      - Multi-backend CoRe interpreter"
echo "  fforge     - Fast JIT-compiled CoRe"
echo "  forger     - CoRe language utility"
echo "  metroman   - Language metrics tool"
echo "  test_parser - Parser testing utility"
echo ""
echo "Usage examples:"
echo "  forge program.fr       # Run with default backend"
echo "  forge -r program.fr    # Run with Rust interpreter"
echo "  forge -j program.fr    # Run with JIT compiler"
echo "  fforge program.fr      # Run with JIT compiler"
echo ""
echo "Test installation:"
echo "  echo 'var x: 42; say: x' | tee test.fr && forge -r test.fr"
