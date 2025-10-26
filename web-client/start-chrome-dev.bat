@echo off
echo Starting Chrome with CORS disabled for development...
echo WARNING: This is for development only - DO NOT use for regular browsing!
echo.

set CHROME_PATH=""
set USER_DATA_DIR=%TEMP%\chrome-dev-session

:: Try to find Chrome in common locations
if exist "C:\Program Files\Google\Chrome\Application\chrome.exe" (
    set CHROME_PATH="C:\Program Files\Google\Chrome\Application\chrome.exe"
) else if exist "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe" (
    set CHROME_PATH="C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"
) else (
    echo Chrome not found. Please install Google Chrome.
    pause
    exit /b 1
)

:: Create a temporary user data directory
if not exist "%USER_DATA_DIR%" mkdir "%USER_DATA_DIR%"

:: Launch Chrome with disabled security for development
echo Opening file:///%~dp0wasm-index.html
%CHROME_PATH% ^
    --user-data-dir="%USER_DATA_DIR%" ^
    --disable-web-security ^
    --disable-features=VizDisplayCompositor ^
    --allow-running-insecure-content ^
    --disable-extensions ^
    --no-first-run ^
    "file:///%~dp0wasm-index.html"

echo.
echo Chrome started with disabled CORS for development.
echo Close Chrome when done testing.
pause