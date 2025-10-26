@echo off
echo Starting Node.js development server with CORS support...
echo.
echo Installing dependencies...
npm init -y >nul 2>&1

echo.
echo Server will be available at:
echo   http://localhost:8000
echo.
echo Press Ctrl+C to stop the server
echo.

cd /d "%~dp0"
node dev-server.js

pause