@echo off
echo Starting development HTTP server with CORS support...
echo.
echo Server will be available at:
echo   http://localhost:8000
echo.
echo Press Ctrl+C to stop the server
echo.

cd /d "%~dp0"
python -m http.server 8000 --bind 127.0.0.1

pause