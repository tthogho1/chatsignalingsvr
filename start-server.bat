@echo off
echo 🚀 Starting WebSocket Chat and Signaling Server...
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
echo ⚙️  Starting server with default configuration:
echo    - Bind Address: 127.0.0.1 (localhost only)
echo    - Port: 8080
echo    - Max Connections: 1000
echo    - Log Level: info
echo.
echo 🌐 WebSocket URL: ws://127.0.0.1:8080
echo 📝 Press Ctrl+C to stop the server
echo.

REM Build and start the server
echo 🔨 Building and starting server...
cargo run --bin websocket-chat-signaling-server

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