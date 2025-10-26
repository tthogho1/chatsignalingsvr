@echo off
echo =================================
echo WebRTC Chat System - Full Test
echo =================================
echo.

echo Starting servers...
echo.

:: Start WebSocket signaling server
echo 1. Starting WebSocket signaling server...
cd /d "%~dp0"
start "Signaling Server" cmd /k "cd /d %~dp0 && start-server.bat"

:: Wait a moment for server to start
timeout /t 3 /nobreak >nul

:: Start web client server
echo 2. Starting web client server...
cd web-client
start "Web Client Server" cmd /k "start-dev-server.bat"

:: Wait a moment for web server to start  
timeout /t 3 /nobreak >nul

:: Open browser
echo 3. Opening browser...
start http://localhost:8000/wasm-fixed.html

echo.
echo ========================================
echo 🚀 System started successfully!
echo.
echo Services:
echo   - Signaling Server: ws://127.0.0.1:8080
echo   - Web Client: http://localhost:8000
echo.
echo Instructions:
echo   1. Enter username and connect
echo   2. Open another browser tab/window
echo   3. Connect with different username  
echo   4. Start video call between users
echo.
echo Press any key to stop all servers...
echo ========================================
pause

echo Stopping servers...
taskkill /F /FI "WindowTitle eq Signaling Server*" 2>nul
taskkill /F /FI "WindowTitle eq Web Client Server*" 2>nul
echo Servers stopped.