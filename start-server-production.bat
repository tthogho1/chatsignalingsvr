@echo off
echo 🚀 Starting WebSocket Chat and Signaling Server (Production Mode)...
echo.

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Cargo is not installed or not in PATH. Please install Rust first:
    echo    Visit: https://rustup.rs/
    pause
    exit /b 1
)

REM Show current directory and configuration
echo 📁 Working directory: %CD%
echo ⚙️  Starting server with production configuration:
echo    - Bind Address: 0.0.0.0 (all interfaces)
echo    - Port: 8080
echo    - Max Connections: 2000
echo    - Log Level: warn
echo    - Build Mode: Release (optimized)
echo.
echo 🌐 WebSocket URLs:
echo    - Local:    ws://127.0.0.1:8080
echo    - Network:  ws://[your-ip]:8080
echo 📝 Press Ctrl+C to stop the server
echo.

REM Build release version and start the server with production settings
echo 🔨 Building release version (this may take a while)...
cargo build --release --bin websocket-chat-signaling-server

if %ERRORLEVEL% NEQ 0 (
    echo ❌ Build failed!
    pause
    exit /b 1
)

echo ✅ Build completed successfully!
echo 🚀 Starting production server...

REM Run the release binary directly for better performance
target\release\websocket-chat-signaling-server.exe ^
  --bind-address 0.0.0.0 ^
  --port 8080 ^
  --max-connections 2000 ^
  --log-level warn

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ❌ Server failed to start!
    echo 💡 Check the error messages above for troubleshooting.
    pause
    exit /b 1
)

echo.
echo 🛑 Server stopped.
pause