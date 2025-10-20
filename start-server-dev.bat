@echo off
echo 🚀 Starting WebSocket Chat and Signaling Server (Development Mode)...
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
echo ⚙️  Starting server with development configuration:
echo    - Bind Address: 0.0.0.0 (all interfaces)
echo    - Port: 8080
echo    - Max Connections: 100
echo    - Log Level: debug
echo.
echo 🌐 WebSocket URLs:
echo    - Local:    ws://127.0.0.1:8080
echo    - Network:  ws://[your-ip]:8080
echo 📝 Press Ctrl+C to stop the server
echo.

REM Build and start the server with development settings
echo 🔨 Building and starting server...
cargo run --bin websocket-chat-signaling-server -- ^
  --bind-address 0.0.0.0 ^
  --port 8080 ^
  --max-connections 100 ^
  --log-level debug

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