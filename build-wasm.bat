@echo off
echo 🦀 Building Rust WASM client...

REM Check if wasm-pack is installed
where wasm-pack >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ wasm-pack is not installed. Please install it first:
    echo    Visit: https://rustwasm.github.io/wasm-pack/installer/
    echo    Or run: cargo install wasm-pack
    pause
    exit /b 1
)

REM Build WASM module
echo 📦 Building WASM package...
cd wasm-client
wasm-pack build --target web --out-dir ../web-client/pkg --out-name wasm_client

if %ERRORLEVEL% EQU 0 (
    cd ..
    echo ✅ WASM build completed successfully!
    echo 📁 Output files are in web-client/pkg/
    echo.
    echo 🚀 To run the WASM client:
    echo    1. cd web-client
    echo    2. python server.py
    echo    3. Open http://localhost:8000/wasm-index.html
    echo.
    echo 📝 Make sure the WebSocket server is running:
    echo    cargo run
) else (
    cd ..
    echo ❌ WASM build failed!
    pause
    exit /b 1
)

pause