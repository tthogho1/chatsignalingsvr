@echo off
echo =================================
echo WebRTC Chat System - WASM Test
echo =================================
echo.

echo Checking if required files exist...

if not exist "wasm-client\pkg\wasm_webrtc_client.js" (
    echo ❌ WASMモジュールが見つかりません
    echo wasm-clientディレクトリで以下を実行してください:
    echo   wasm-pack build --target web --dev
    pause
    exit /b 1
)

if not exist "web-client\wasm-fixed.html" (
    echo ❌ wasm-fixed.htmlが見つかりません
    pause
    exit /b 1
)

echo ✅ 必要なファイルが見つかりました
echo.

echo サーバーを起動しています...
echo.

echo 1. WebSocketシグナリングサーバーを起動中...
start /min cmd /c "cargo run --bin websocket-chat-signaling-server 2>&1"

echo 2. HTTPウェブサーバーを起動中...
timeout /t 2 >nul
start /min cmd /c "python -m http.server 8000 2>&1"

echo 3. ブラウザーを開いています...
timeout /t 3 >nul
start http://localhost:8000/web-client/wasm-fixed.html

echo.
echo ========================================
echo ✅ システムが正常に起動しました！
echo.
echo Services:
echo   - シグナリングサーバー: ws://127.0.0.1:8080
echo   - WASMクライアント: http://localhost:8000/web-client/wasm-fixed.html
echo.
echo 使用方法:
echo   1. ユーザー名を入力して接続
echo   2. 別のブラウザータブ/ウィンドウを開く
echo   3. 別のユーザー名で接続
echo   4. ユーザー間でWebRTC通話を開始
echo.
echo ========================================
echo.
echo サーバーを停止するには何かキーを押してください...
pause >nul

echo サーバーを停止しています...
echo.

for /f "tokens=5" %%a in ('netstat -ano ^| findstr :8080') do (
    if not "%%a"=="0" (
        echo WebSocketサーバー (PID: %%a) を停止中...
        taskkill /F /PID %%a >nul 2>&1
    )
)

for /f "tokens=5" %%a in ('netstat -ano ^| findstr :8000') do (
    if not "%%a"=="0" (
        echo HTTPサーバー (PID: %%a) を停止中...
        taskkill /F /PID %%a >nul 2>&1
    )
)

echo ✅ サーバーが停止されました