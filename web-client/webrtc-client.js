// WebRTC Video Chat Client
class VideoChat {
  constructor() {
    this.ws = null;
    this.localStream = null;
    this.remoteStream = null;
    this.peerConnection = null;
    this.username = null;
    this.currentCall = null;
    this.isConnected = false;
    this.isInCall = false;
    this.isCameraOn = true;
    this.isMicOn = true;

    // WebRTC configuration
    this.rtcConfig = {
      iceServers: [
        { urls: 'stun:stun.l.google.com:19302' },
        { urls: 'stun:stun1.l.google.com:19302' },
      ],
    };

    this.initializeElements();
    this.setupEventListeners();
    this.initializeMediaDevices();
  }

  // Initialize DOM elements
  initializeElements() {
    // Connection elements
    this.connectionPanel = document.getElementById('connection-panel');
    this.mainContent = document.getElementById('main-content');
    this.serverUrlInput = document.getElementById('server-url');
    this.usernameInput = document.getElementById('username');
    this.connectBtn = document.getElementById('connect-btn');
    this.disconnectBtn = document.getElementById('disconnect-btn');
    this.usernameDisplay = document.getElementById('username-display');

    // Video elements
    this.localVideo = document.getElementById('local-video');
    this.remoteVideo = document.getElementById('remote-video');
    this.toggleCameraBtn = document.getElementById('toggle-camera');
    this.toggleMicBtn = document.getElementById('toggle-mic');

    // Call control elements
    this.targetUserInput = document.getElementById('target-user');
    this.startCallBtn = document.getElementById('start-call');
    this.endCallBtn = document.getElementById('end-call');

    // Chat elements
    this.chatMessages = document.getElementById('chat-messages');
    this.messageInput = document.getElementById('message-input');
    this.sendMessageBtn = document.getElementById('send-message');
    this.sendBroadcastBtn = document.getElementById('send-broadcast');
    this.clearChatBtn = document.getElementById('clear-chat');

    // Status elements
    this.connectionStatus = document.getElementById('connection-status');
    this.callStatus = document.getElementById('call-status');
    this.onlineUsers = document.getElementById('online-users');

    // Modal elements
    this.incomingCallModal = document.getElementById('incoming-call-modal');
    this.callerName = document.getElementById('caller-name');
    this.acceptCallBtn = document.getElementById('accept-call');
    this.rejectCallBtn = document.getElementById('reject-call');

    // Notification area
    this.notifications = document.getElementById('notifications');
  }

  // Setup event listeners
  setupEventListeners() {
    // Connection events
    this.connectBtn.addEventListener('click', () => this.connect());
    this.disconnectBtn.addEventListener('click', () => this.disconnect());

    // Media control events
    this.toggleCameraBtn.addEventListener('click', () => this.toggleCamera());
    this.toggleMicBtn.addEventListener('click', () => this.toggleMicrophone());

    // Call control events
    this.startCallBtn.addEventListener('click', () => this.initiateCall());
    this.endCallBtn.addEventListener('click', () => this.endCall());

    // Chat events
    this.sendMessageBtn.addEventListener('click', () => this.sendMessage());
    this.sendBroadcastBtn.addEventListener('click', () => this.sendBroadcast());
    this.clearChatBtn.addEventListener('click', () => this.clearChat());
    this.messageInput.addEventListener('keypress', e => {
      if (e.key === 'Enter') {
        this.sendMessage();
      }
    });

    // Modal events
    this.acceptCallBtn.addEventListener('click', () => this.acceptCall());
    this.rejectCallBtn.addEventListener('click', () => this.rejectCall());

    // Username input events
    this.usernameInput.addEventListener('keypress', e => {
      if (e.key === 'Enter') {
        this.connect();
      }
    });
  }

  // Initialize media devices
  async initializeMediaDevices() {
    try {
      this.localStream = await navigator.mediaDevices.getUserMedia({
        video: { width: 640, height: 480 },
        audio: true,
      });
      this.localVideo.srcObject = this.localStream;
      this.showNotification('メディアデバイスの準備完了', 'success');
    } catch (error) {
      console.error('Media devices error:', error);
      this.showNotification('カメラ・マイクへのアクセスに失敗しました', 'error');
    }
  }

  // Connect to WebSocket server
  async connect() {
    const serverUrl = this.serverUrlInput.value.trim();
    const username = this.usernameInput.value.trim();

    if (!serverUrl || !username) {
      this.showNotification('サーバーURLとユーザー名を入力してください', 'warning');
      return;
    }

    try {
      this.ws = new WebSocket(serverUrl);
      this.username = username;

      this.ws.onopen = () => {
        this.isConnected = true;
        this.updateConnectionStatus('connected');
        this.usernameDisplay.textContent = this.username;
        this.connectionPanel.style.display = 'none';
        this.mainContent.style.display = 'block';
        this.disconnectBtn.disabled = false;
        this.showNotification('サーバーに接続しました', 'success');
      };

      this.ws.onmessage = event => {
        this.handleWebSocketMessage(event);
      };

      this.ws.onclose = () => {
        this.isConnected = false;
        this.updateConnectionStatus('disconnected');
        this.showNotification('サーバーから切断されました', 'warning');
        this.resetUI();
      };

      this.ws.onerror = error => {
        console.error('WebSocket error:', error);
        this.showNotification('接続エラーが発生しました', 'error');
      };
    } catch (error) {
      console.error('Connection error:', error);
      this.showNotification('接続に失敗しました', 'error');
    }
  }

  // Disconnect from server
  disconnect() {
    if (this.ws) {
      this.ws.close();
    }
    this.endCall();
    this.resetUI();
  }

  // Handle WebSocket messages
  handleWebSocketMessage(event) {
    try {
      const message = JSON.parse(event.data);

      switch (message.message_type?.type || message.message_type) {
        case 'TextChat':
          this.handleTextMessage(message);
          break;
        case 'Broadcast':
          this.handleBroadcastMessage(message);
          break;
        case 'WebRTCSignaling':
          this.handleSignalingMessage(message);
          break;
        case 'Generic':
          this.handleGenericMessage(message);
          break;
        default:
          console.log('Unknown message type:', message);
      }
    } catch (error) {
      console.error('Message parsing error:', error);
    }
  }

  // Handle text chat messages
  handleTextMessage(message) {
    const isOwn = message.sender_id === this.username;
    this.addChatMessage(
      message.message_type.content,
      message.sender_id || 'System',
      isOwn ? 'own' : 'other',
      new Date(message.timestamp)
    );
  }

  // Handle broadcast messages
  handleBroadcastMessage(message) {
    const isOwn = message.sender_id === this.username;
    this.addChatMessage(
      message.message_type.content,
      message.sender_id || 'System',
      isOwn ? 'own' : 'broadcast',
      new Date(message.timestamp)
    );
  }

  // Handle WebRTC signaling messages
  async handleSignalingMessage(message) {
    const { signaling_data } = message.message_type;
    const senderId = message.sender_id;

    console.log('Received signaling:', signaling_data.type, 'from:', senderId);

    switch (signaling_data.type) {
      case 'offer':
        await this.handleOffer(signaling_data, senderId);
        break;
      case 'answer':
        await this.handleAnswer(signaling_data);
        break;
      case 'ice-candidate':
        await this.handleIceCandidate(signaling_data);
        break;
      case 'call-request':
        this.handleIncomingCall(senderId);
        break;
      case 'call-accepted':
        await this.startPeerConnection(senderId);
        break;
      case 'call-rejected':
        this.showNotification(`${senderId}さんが通話を拒否しました`, 'warning');
        this.updateCallStatus('rejected');
        break;
      case 'call-ended':
        this.handleCallEnded();
        break;
    }
  }

  // Handle generic messages
  handleGenericMessage(message) {
    this.addChatMessage(
      JSON.stringify(message.message_type.content),
      message.sender_id || 'System',
      'system',
      new Date(message.timestamp)
    );
  }

  // Send WebSocket message
  sendWebSocketMessage(messageType, content = null) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      this.showNotification('サーバーに接続されていません', 'error');
      return;
    }

    const message = {
      id: this.generateUUID(),
      sender_id: this.username,
      timestamp: new Date().toISOString(),
      message_type: messageType,
    };

    this.ws.send(JSON.stringify(message));
  }

  // Send text message
  sendMessage() {
    const content = this.messageInput.value.trim();
    const targetUser = this.targetUserInput.value.trim();

    if (!content) return;

    if (targetUser) {
      // Direct message (not implemented in server yet, but structure is ready)
      this.sendWebSocketMessage({
        type: 'TextChat',
        content: content,
        target_user: targetUser,
      });
    } else {
      this.sendWebSocketMessage({
        type: 'TextChat',
        content: content,
      });
    }

    this.messageInput.value = '';
  }

  // Send broadcast message
  sendBroadcast() {
    const content = this.messageInput.value.trim();
    if (!content) return;

    this.sendWebSocketMessage({
      type: 'Broadcast',
      content: content,
    });

    this.messageInput.value = '';
  }

  // Send signaling message
  sendSignaling(targetUser, signalingData) {
    this.sendWebSocketMessage({
      type: 'WebRTCSignaling',
      target_user_id: targetUser,
      signaling_data: signalingData,
    });
  }

  // Initiate call
  async initiateCall() {
    const targetUser = this.targetUserInput.value.trim();
    if (!targetUser) {
      this.showNotification('通話相手のユーザー名を入力してください', 'warning');
      return;
    }

    if (targetUser === this.username) {
      this.showNotification('自分自身には発信できません', 'warning');
      return;
    }

    this.currentCall = targetUser;
    this.updateCallStatus('calling');

    // Send call request
    this.sendSignaling(targetUser, {
      type: 'call-request',
    });

    this.showNotification(`${targetUser}さんに発信中...`, 'info');
  }

  // Handle incoming call
  handleIncomingCall(caller) {
    this.currentCall = caller;
    this.callerName.textContent = caller;
    this.incomingCallModal.style.display = 'block';
    this.updateCallStatus('incoming');
  }

  // Accept incoming call
  async acceptCall() {
    this.incomingCallModal.style.display = 'none';

    // Send acceptance
    this.sendSignaling(this.currentCall, {
      type: 'call-accepted',
    });

    await this.startPeerConnection(this.currentCall, false);
    this.updateCallStatus('connected');
    this.showNotification(`${this.currentCall}さんとの通話を開始しました`, 'success');
  }

  // Reject incoming call
  rejectCall() {
    this.incomingCallModal.style.display = 'none';

    // Send rejection
    this.sendSignaling(this.currentCall, {
      type: 'call-rejected',
    });

    this.currentCall = null;
    this.updateCallStatus('idle');
  }

  // Start peer connection
  async startPeerConnection(targetUser, isInitiator = true) {
    try {
      this.peerConnection = new RTCPeerConnection(this.rtcConfig);

      // Add local stream
      if (this.localStream) {
        this.localStream.getTracks().forEach(track => {
          this.peerConnection.addTrack(track, this.localStream);
        });
      }

      // Handle remote stream
      this.peerConnection.ontrack = event => {
        this.remoteStream = event.streams[0];
        this.remoteVideo.srcObject = this.remoteStream;
        document.getElementById('remote-label').textContent = `${targetUser}`;
      };

      // Handle ICE candidates
      this.peerConnection.onicecandidate = event => {
        if (event.candidate) {
          this.sendSignaling(targetUser, {
            type: 'ice-candidate',
            candidate: event.candidate,
          });
        }
      };

      if (isInitiator) {
        // Create and send offer
        const offer = await this.peerConnection.createOffer();
        await this.peerConnection.setLocalDescription(offer);

        this.sendSignaling(targetUser, {
          type: 'offer',
          sdp: offer,
        });
      }

      this.isInCall = true;
      this.startCallBtn.style.display = 'none';
      this.endCallBtn.style.display = 'inline-block';
    } catch (error) {
      console.error('Peer connection error:', error);
      this.showNotification('通話の開始に失敗しました', 'error');
    }
  }

  // Handle offer
  async handleOffer(signalingData, senderId) {
    try {
      if (!this.peerConnection) {
        await this.startPeerConnection(senderId, false);
      }

      await this.peerConnection.setRemoteDescription(signalingData.sdp);
      const answer = await this.peerConnection.createAnswer();
      await this.peerConnection.setLocalDescription(answer);

      this.sendSignaling(senderId, {
        type: 'answer',
        sdp: answer,
      });
    } catch (error) {
      console.error('Handle offer error:', error);
    }
  }

  // Handle answer
  async handleAnswer(signalingData) {
    try {
      await this.peerConnection.setRemoteDescription(signalingData.sdp);
      this.updateCallStatus('connected');
      this.showNotification('通話が接続されました', 'success');
    } catch (error) {
      console.error('Handle answer error:', error);
    }
  }

  // Handle ICE candidate
  async handleIceCandidate(signalingData) {
    try {
      await this.peerConnection.addIceCandidate(signalingData.candidate);
    } catch (error) {
      console.error('Handle ICE candidate error:', error);
    }
  }

  // End call
  endCall() {
    if (this.currentCall) {
      this.sendSignaling(this.currentCall, {
        type: 'call-ended',
      });
    }

    this.handleCallEnded();
  }

  // Handle call ended
  handleCallEnded() {
    if (this.peerConnection) {
      this.peerConnection.close();
      this.peerConnection = null;
    }

    this.remoteVideo.srcObject = null;
    document.getElementById('remote-label').textContent = '相手（未接続）';

    this.isInCall = false;
    this.currentCall = null;
    this.updateCallStatus('idle');

    this.startCallBtn.style.display = 'inline-block';
    this.endCallBtn.style.display = 'none';
    this.incomingCallModal.style.display = 'none';

    this.showNotification('通話が終了しました', 'info');
  }

  // Toggle camera
  toggleCamera() {
    if (this.localStream) {
      const videoTrack = this.localStream.getVideoTracks()[0];
      if (videoTrack) {
        videoTrack.enabled = !videoTrack.enabled;
        this.isCameraOn = videoTrack.enabled;
        this.toggleCameraBtn.textContent = this.isCameraOn ? '📹' : '📹❌';
        this.toggleCameraBtn.classList.toggle('btn-danger', !this.isCameraOn);
        this.toggleCameraBtn.classList.toggle('btn-secondary', this.isCameraOn);
      }
    }
  }

  // Toggle microphone
  toggleMicrophone() {
    if (this.localStream) {
      const audioTrack = this.localStream.getAudioTracks()[0];
      if (audioTrack) {
        audioTrack.enabled = !audioTrack.enabled;
        this.isMicOn = audioTrack.enabled;
        this.toggleMicBtn.textContent = this.isMicOn ? '🎤' : '🎤❌';
        this.toggleMicBtn.classList.toggle('btn-danger', !this.isMicOn);
        this.toggleMicBtn.classList.toggle('btn-secondary', this.isMicOn);
      }
    }
  }

  // Add chat message
  addChatMessage(content, sender, type, timestamp) {
    const messageDiv = document.createElement('div');
    messageDiv.className = `message ${type}`;

    const senderDiv = document.createElement('div');
    senderDiv.className = 'message-sender';
    senderDiv.textContent = sender;

    const contentDiv = document.createElement('div');
    contentDiv.textContent = content;

    const timeDiv = document.createElement('div');
    timeDiv.className = 'message-time';
    timeDiv.textContent = timestamp.toLocaleTimeString();

    messageDiv.appendChild(senderDiv);
    messageDiv.appendChild(contentDiv);
    messageDiv.appendChild(timeDiv);

    this.chatMessages.appendChild(messageDiv);
    this.chatMessages.scrollTop = this.chatMessages.scrollHeight;
  }

  // Clear chat
  clearChat() {
    this.chatMessages.innerHTML = '';
  }

  // Update connection status
  updateConnectionStatus(status) {
    this.connectionStatus.textContent = status === 'connected' ? '接続済み' : '未接続';
    this.connectionStatus.style.color = status === 'connected' ? '#27ae60' : '#e74c3c';
  }

  // Update call status
  updateCallStatus(status) {
    const statusMap = {
      idle: '待機中',
      calling: '発信中',
      incoming: '着信中',
      connected: '通話中',
      rejected: '拒否',
    };

    this.callStatus.textContent = statusMap[status] || status;
    this.callStatus.style.color =
      status === 'connected' ? '#27ae60' : status === 'rejected' ? '#e74c3c' : '#f39c12';
  }

  // Show notification
  showNotification(message, type = 'info') {
    const notification = document.createElement('div');
    notification.className = `notification ${type}`;
    notification.textContent = message;

    this.notifications.appendChild(notification);

    // Auto remove after 5 seconds
    setTimeout(() => {
      if (notification.parentNode) {
        notification.parentNode.removeChild(notification);
      }
    }, 5000);
  }

  // Reset UI
  resetUI() {
    this.connectionPanel.style.display = 'block';
    this.mainContent.style.display = 'none';
    this.disconnectBtn.disabled = true;
    this.usernameDisplay.textContent = '未接続';
    this.endCall();
    this.updateConnectionStatus('disconnected');
    this.updateCallStatus('idle');
  }

  // Generate UUID
  generateUUID() {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
      const r = (Math.random() * 16) | 0;
      const v = c == 'x' ? r : (r & 0x3) | 0x8;
      return v.toString(16);
    });
  }
}

// Initialize the video chat application
document.addEventListener('DOMContentLoaded', () => {
  new VideoChat();
});
