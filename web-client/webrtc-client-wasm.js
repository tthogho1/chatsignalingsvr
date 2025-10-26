// WASM WebRTC Video Chat Client
import init, { VideoChat } from './pkg/wasm_webrtc_client.js';

let wasmVideoChat = null;

// Initialize WASM module and setup
async function initWasmVideoChat() {
  try {
    console.log('🚀 Initializing WASM WebRTC Client...');
    await init();
    console.log('✅ WASM module loaded successfully');

    wasmVideoChat = new VideoChat();
    await wasmVideoChat.initialize();
    console.log('✅ WASM VideoChat initialized');

    setupUI();
    console.log('✅ UI setup complete');
  } catch (error) {
    console.error('❌ WASMモジュールの読み込みに失敗しました');
    console.error('エラー:', error.message);
    console.error('HTTPサーバーから実行してください');

    // Show error message to user
    showError(`WASM初期化エラー: ${error.message}`);
  }
}

// Setup UI event listeners
function setupUI() {
  const connectBtn = document.getElementById('connect-btn');
  const disconnectBtn = document.getElementById('disconnect-btn');
  const startCallBtn = document.getElementById('start-call');
  const endCallBtn = document.getElementById('end-call');
  const toggleCameraBtn = document.getElementById('toggle-camera');
  const toggleMicBtn = document.getElementById('toggle-mic');

  // Connection
  connectBtn.addEventListener('click', async () => {
    const serverUrl = document.getElementById('server-url').value;
    const username = document.getElementById('username').value;

    if (!username.trim()) {
      showError('ユーザー名を入力してください');
      return;
    }

    try {
      console.log(`Connecting to ${serverUrl} as ${username}`);
      await wasmVideoChat.connect(serverUrl, username);
      console.log('✅ Connected successfully');

      // Update UI
      document.getElementById('connection-panel').style.display = 'none';
      document.getElementById('main-content').style.display = 'block';
      document.getElementById('username-display').textContent = username;
      connectBtn.disabled = true;
      disconnectBtn.disabled = false;

      showSuccess(`${username} として接続しました`);
    } catch (error) {
      console.error('❌ Connection failed:', error);
      showError(`接続失敗: ${error.message}`);
    }
  });

  // Disconnect
  disconnectBtn.addEventListener('click', () => {
    try {
      wasmVideoChat.disconnect();

      // Update UI
      document.getElementById('connection-panel').style.display = 'block';
      document.getElementById('main-content').style.display = 'none';
      document.getElementById('username-display').textContent = '未接続';
      connectBtn.disabled = false;
      disconnectBtn.disabled = true;

      showSuccess('切断しました');
    } catch (error) {
      console.error('❌ Disconnect failed:', error);
      showError(`切断失敗: ${error.message}`);
    }
  });

  // Start call
  startCallBtn.addEventListener('click', async () => {
    const targetUser = document.getElementById('target-user').value;

    if (!targetUser.trim()) {
      showError('通話相手のユーザー名を入力してください');
      return;
    }

    try {
      console.log(`Starting call to ${targetUser}`);
      await wasmVideoChat.start_call(targetUser);
      console.log('✅ Call started successfully');

      startCallBtn.disabled = true;
      endCallBtn.disabled = false;

      showSuccess(`${targetUser} に通話を開始しました`);
    } catch (error) {
      console.error('❌ Call start failed:', error);
      showError(`通話開始失敗: ${error.message}`);
    }
  });

  // End call
  endCallBtn.addEventListener('click', () => {
    try {
      wasmVideoChat.end_call();

      startCallBtn.disabled = false;
      endCallBtn.disabled = true;

      showSuccess('通話を終了しました');
    } catch (error) {
      console.error('❌ Call end failed:', error);
      showError(`通話終了失敗: ${error.message}`);
    }
  });

  // Toggle camera
  toggleCameraBtn.addEventListener('click', async () => {
    try {
      const isOn = await wasmVideoChat.toggle_camera();
      toggleCameraBtn.textContent = isOn ? '📹' : '📹❌';
      toggleCameraBtn.classList.toggle('btn-secondary', isOn);
      toggleCameraBtn.classList.toggle('btn-danger', !isOn);
    } catch (error) {
      console.error('❌ Camera toggle failed:', error);
      showError(`カメラ切り替え失敗: ${error.message}`);
    }
  });

  // Toggle microphone
  toggleMicBtn.addEventListener('click', async () => {
    try {
      const isOn = await wasmVideoChat.toggle_microphone();
      toggleMicBtn.textContent = isOn ? '🎤' : '🎤❌';
      toggleMicBtn.classList.toggle('btn-secondary', isOn);
      toggleMicBtn.classList.toggle('btn-danger', !isOn);
    } catch (error) {
      console.error('❌ Microphone toggle failed:', error);
      showError(`マイク切り替え失敗: ${error.message}`);
    }
  });
}

// Notification functions
function showError(message) {
  showNotification(message, 'error');
}

function showSuccess(message) {
  showNotification(message, 'success');
}

function showNotification(message, type = 'info') {
  const notifications = document.getElementById('notifications');
  const notification = document.createElement('div');
  notification.className = `notification notification-${type}`;
  notification.textContent = message;

  notifications.appendChild(notification);

  // Auto remove after 5 seconds
  setTimeout(() => {
    if (notification.parentNode) {
      notification.parentNode.removeChild(notification);
    }
  }, 5000);
}

// Initialize when page loads
document.addEventListener('DOMContentLoaded', initWasmVideoChat);
