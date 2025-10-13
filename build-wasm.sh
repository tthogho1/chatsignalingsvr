#!/bin/bash

# WASM client build script

echo "🦀 Building Rust WASM client..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack is not installed. Installing..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build WASM module
echo "📦 Building WASM package..."
wasm-pack build --target web --out-dir web-client/pkg --out-name wasm_client

if [ $? -eq 0 ]; then
    echo "✅ WASM build completed successfully!"
    echo "📁 Output files are in web-client/pkg/"
    echo ""
    echo "🚀 To run the WASM client:"
    echo "   1. cd web-client"
    echo "   2. python server.py"
    echo "   3. Open http://localhost:8000/wasm-index.html"
    echo ""
    echo "📝 Make sure the WebSocket server is running:"
    echo "   cargo run"
else
    echo "❌ WASM build failed!"
    exit 1
fi