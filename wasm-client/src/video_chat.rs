use wasm_bindgen::prelude::*;
use web_sys::{console, MediaStream};
use std::rc::Rc;
use std::cell::RefCell;

use crate::websocket_client::WebSocketClient;
use crate::webrtc_client::WebRTCClient;
use crate::dom_helpers::DomHelpers;

// WASM client entry point
pub struct VideoChat {
    websocket: Rc<RefCell<Option<WebSocketClient>>>,
    webrtc: Rc<RefCell<Option<WebRTCClient>>>,
    dom: DomHelpers,
    username: Option<String>,
    current_call: Option<String>,
    is_connected: bool,
    is_in_call: bool,
}

impl VideoChat {
    pub fn new() -> VideoChat {
        console_error_panic_hook::set_once();
        VideoChat {
            websocket: Rc::new(RefCell::new(None)),
            webrtc: Rc::new(RefCell::new(None)),
            dom: DomHelpers::new().unwrap(),
            username: None,
            current_call: None,
            is_connected: false,
            is_in_call: false,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), JsValue> {
        console::log_1(&"Initializing VideoChat WASM client...".into());
        // Initialize WebRTC client
        let webrtc_client = WebRTCClient::new(self.clone_handle_rc()).await?;
        *self.webrtc.borrow_mut() = Some(webrtc_client);
        // Setup DOM event listeners
        self.dom.setup_event_listeners(self)?;
        console::log_1(&"VideoChat WASM client initialized successfully".into());
        Ok(())
    }

    pub async fn connect(&mut self, url: &str, username: &str) -> Result<(), JsValue> {
        console::log_2(&"Connecting to".into(), &url.into());
        
        let websocket_client = WebSocketClient::new(url, username, self.clone_handle()).await?;
        *self.websocket.borrow_mut() = Some(websocket_client);
        
        self.username = Some(username.to_string());
        self.is_connected = true;
        
        // Update UI
        self.dom.update_connection_status(true, username)?;
        
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), JsValue> {
        if let Some(ref mut websocket) = &mut *self.websocket.borrow_mut() {
            websocket.close()?;
        }
        *self.websocket.borrow_mut() = None;
        
        self.username = None;
        self.is_connected = false;
        
        // Update UI
        self.dom.update_connection_status(false, "")?;
        
        Ok(())
    }

    pub async fn start_call(&mut self, target_user: &str) -> Result<(), JsValue> {
        if !self.is_connected {
            return Err(JsValue::from_str("Not connected to server"));
        }

        // 通話開始の詳細ログ
        console::group_1(&"=== Starting WebRTC Call ===".into());
        console::log_2(&"Caller:".into(), &self.username.as_ref().unwrap_or(&"unknown".to_string()).into());
        console::log_2(&"Target:".into(), &target_user.into());
        
        // Initialize new WebRTC client for this call (in case of reconnection)
        console::log_1(&"Initializing fresh WebRTC client...".into());
        let webrtc_client = WebRTCClient::new(self.clone_handle_rc()).await?;
        *self.webrtc.borrow_mut() = Some(webrtc_client);
        
        // ICE候補イベントリスナーを設定
        self.setup_ice_candidate_listener(target_user)?;
        
        console::log_1(&"Getting user media...".into());
        
        // Get user media
        if let Some(ref webrtc) = &*self.webrtc.borrow() {
            let local_stream = webrtc.get_user_media().await?;
            console::log_1(&"User media obtained successfully".into());
            console::log_1(&"Setting local video stream to DOM...".into());
                // Set local video stream to DOM
                if let Some(ref webrtc) = &*self.webrtc.borrow() {
                    if let Some(local_stream) = webrtc.get_local_stream() {
                        self.dom.set_local_video_stream(&local_stream)?;
                    }
                }
            console::log_1(&"✅ Local video stream set successfully".into());
            
            // Create offer (SDP)
            console::log_1(&"Creating WebRTC offer...".into());
            let offer = webrtc.create_offer().await?;
            console::log_2(&"Offer created, SDP length:".into(), &offer.len().into());
            
            // Send offer through WebSocket
            if let Some(ref websocket) = &*self.websocket.borrow() {
                console::log_1(&"Sending offer to server...".into());
                websocket.send_signaling_message(target_user, "offer", &offer).await?;
                console::log_1(&"Offer sent successfully".into());
            }
        }
        console::group_end();
        
        self.current_call = Some(target_user.to_string());
        self.is_in_call = true;
        
        // Update UI
        self.dom.update_call_status(true, target_user)?;
        
        Ok(())
    }

    pub fn end_call(&mut self) -> Result<(), JsValue> {
        console::group_1(&"=== Ending WebRTC Call ===".into());
        console::log_2(&"Current call with:".into(), &self.current_call.clone().unwrap_or("unknown".to_string()).into());
        
        // Close peer connection
        if let Some(ref webrtc) = &*self.webrtc.borrow() {
            console::log_1(&"Closing WebRTC peer connection".into());
            webrtc.close_connection()?;
        }
        
        // Notify other party
        if let Some(ref current_call) = &self.current_call {
            if let Some(ref websocket) = &*self.websocket.borrow() {
                console::log_2(&"Notifying remote peer of hangup:".into(), &current_call.into());
                let _ = websocket.send_signaling_message(current_call, "hangup", "");
            }
        }
        
        self.current_call = None;
        self.is_in_call = false;
        console::log_1(&"Call state cleared".into());
        
        // Update UI
        self.dom.update_call_status(false, "")?;
        self.dom.clear_remote_video()?;
        console::group_end();
        
        Ok(())
    }

    pub async fn send_message(&self, message: &str, target: Option<String>) -> Result<(), JsValue> {
        if let Some(ref websocket) = &*self.websocket.borrow() {
            match target {
                Some(target_user) => {
                    websocket.send_direct_message(&target_user, message).await?;
                }
                None => {
                    websocket.send_broadcast_message(message).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_camera(&mut self) -> Result<(), JsValue> {
        if let Some(ref mut webrtc) = &mut *self.webrtc.borrow_mut() {
            let enabled = webrtc.toggle_camera().await?;
            self.dom.update_camera_button(enabled)?;
        }
        Ok(())
    }

    pub async fn toggle_microphone(&mut self) -> Result<(), JsValue> {
        if let Some(ref mut webrtc) = &mut *self.webrtc.borrow_mut() {
            let enabled = webrtc.toggle_microphone().await?;
            self.dom.update_microphone_button(enabled)?;
        }
        Ok(())
    }

    // Internal methods
    // Internal-only methods (not exposed to JS)
    // Internal-only methods (not exposed to JS)
    pub fn clone_handle(&self) -> VideoChat {
        VideoChat {
            websocket: self.websocket.clone(),
            webrtc: self.webrtc.clone(),
            dom: self.dom.clone(),
            username: self.username.clone(),
            current_call: self.current_call.clone(),
            is_connected: self.is_connected,
            is_in_call: self.is_in_call,
        }
    }

    // Internal-only: do not expose to JS
    pub fn clone_handle_rc(&self) -> Rc<RefCell<VideoChat>> {
        Rc::new(RefCell::new(self.clone_handle()))
    }

    pub async fn handle_signaling_message(&mut self, from: &str, signaling_type: &str, data: &str) -> Result<(), JsValue> {
        // WebRTCシグナリング処理の詳細ログ
        console::group_1(&"=== Handling WebRTC Signaling ===".into());
        console::log_2(&"From:".into(), &from.into());
        console::log_2(&"Type:".into(), &signaling_type.into());
        console::log_2(&"Data length:".into(), &data.len().into());
        console::log_2(&"Current user:".into(), &self.username.as_ref().unwrap_or(&"unknown".to_string()).into());
        
        match signaling_type {
            "offer" => {
                console::log_1(&"=== OFFER PROCESSING START ===".into());
                console::log_1(&"Processing incoming call offer".into());
                console::log_2(&"SDP preview (100 chars):".into(), &data.chars().take(100).collect::<String>().into());
                
                console::log_1(&"About to show incoming call dialog...".into());
                // Handle incoming call
                let should_accept = self.dom.show_incoming_call_dialog(from)?;
                console::log_2(&"Dialog result:".into(), &should_accept.into());
                
                if should_accept {
                    console::log_1(&"Call accepted, setting up WebRTC connection".into());
                    
                    // ICE候補イベントリスナーを設定
                    self.setup_ice_candidate_listener(from)?;
                    
                    if let Some(ref webrtc) = &*self.webrtc.borrow() {
                        let local_stream = webrtc.get_user_media().await?;
                        console::log_1(&"Local stream obtained".into());
                        console::log_1(&"Setting local video stream to DOM (answer side)...".into());
                        self.dom.set_local_video_stream(&local_stream)?;
                        console::log_1(&"✅ Local video stream set successfully (answer side)".into());
                        
                        console::log_1(&"Processing offer and creating answer".into());
                        let answer = webrtc.handle_offer(data).await?;
                        console::log_2(&"Answer created, SDP length:".into(), &answer.len().into());
                        
                        if let Some(ref websocket) = &*self.websocket.borrow() {
                            console::log_1(&"Sending answer to caller".into());
                            websocket.send_signaling_message(from, "answer", &answer).await?;
                        }
                        
                        self.current_call = Some(from.to_string());
                        self.is_in_call = true;
                        self.dom.update_call_status(true, from)?;
                    }
                } else {
                    console::log_1(&"=== CALL REJECTED BY USER ===".into());
                    console::log_1(&"Call rejected by user".into());
                    // Reject call
                    if let Some(ref websocket) = &*self.websocket.borrow() {
                        console::log_1(&"Sending reject message...".into());
                        websocket.send_signaling_message(from, "reject", "").await?;
                        console::log_1(&"Reject message sent successfully".into());
                    } else {
                        console::log_1(&"ERROR: No websocket available to send reject".into());
                    }
                }
            }
            "answer" => {
                console::log_1(&"Processing call answer from callee".into());
                console::log_2(&"SDP preview (100 chars):".into(), &data.chars().take(100).collect::<String>().into());
                if let Some(ref webrtc) = &*self.webrtc.borrow() {
                    webrtc.handle_answer(data).await?;
                    console::log_1(&"Answer processed successfully".into());
                }
            }
            "ice-candidate" => {
                console::log_1(&"Processing ICE candidate".into());
                console::log_2(&"Candidate data:".into(), &data.into());
                if let Some(ref webrtc) = &*self.webrtc.borrow() {
                    webrtc.handle_ice_candidate(data).await?;
                    console::log_1(&"ICE candidate processed successfully".into());
                }
            }
            "hangup" => {
                console::log_1(&"Call ended by remote peer".into());
                self.end_call()?;
            }
            "reject" => {
                console::log_1(&"Call was rejected by remote peer".into());
                self.dom.show_notification("Call rejected", "warning")?;
                self.end_call()?;
            }
            _ => {
                console::log_2(&"Unknown signaling type:".into(), &signaling_type.into());
            }
        }
        console::group_end();
        
        Ok(())
    }

    pub fn handle_message(&self, from: &str, message: &str, is_broadcast: bool) -> Result<(), JsValue> {
        let message_type = if is_broadcast { "broadcast" } else { "direct" };
        self.dom.add_chat_message(from, message, message_type)?;
        Ok(())
    }

    // #[wasm_bindgen] を削除
    pub fn handle_remote_stream(&self, stream: &MediaStream) -> Result<(), JsValue> {
        self.dom.set_remote_video_stream(stream)?;
        Ok(())
    }

    /// Test method to send signaling messages directly (exposed to WASM)
    pub async fn send_signaling_message(&self, target_user: &str, signaling_type: &str, data: &str) -> Result<(), JsValue> {
        if !self.is_connected {
            return Err(JsValue::from_str("Not connected to server"));
        }

        console::group_1(&"=== WASM send_signaling_message ===".into());
        console::log_2(&"Target:".into(), &target_user.into());
        console::log_2(&"Signaling Type:".into(), &signaling_type.into());
        console::log_2(&"Data:".into(), &data.into());

        if let Some(ref websocket) = &*self.websocket.borrow() {
            console::log_1(&"Calling WebSocket client send_signaling_message...".into());
            websocket.send_signaling_message(target_user, signaling_type, data).await?;
            console::log_1(&"✅ Signaling message sent successfully".into());
        } else {
            console::error_1(&"❌ WebSocket client not available".into());
            return Err(JsValue::from_str("WebSocket client not available"));
        }

        console::group_end();
        Ok(())
    }

    /// ICE候補イベントリスナーを設定（将来の実装）
    fn setup_ice_candidate_listener(&self, target_user: &str) -> Result<(), JsValue> {
        console::log_2(&"Setting up ICE candidate listener for:".into(), &target_user.into());
        
        if let Some(ref webrtc) = &*self.webrtc.borrow() {
            let websocket_ref = self.websocket.clone();
            let target_user_clone = target_user.to_string();
            
            webrtc.set_ice_candidate_callback(move |candidate: &str| {
                console::log_1(&"ICE candidate callback triggered".into());
                console::log_2(&"Candidate:".into(), &candidate.into());
                
                if let Some(ref websocket) = &*websocket_ref.borrow() {
                    console::log_1(&"Sending ICE candidate to signaling server".into());
                    
                    // 非同期呼び出しをspawn_localで実行
                    let websocket_clone = websocket.clone();
                    let target_clone = target_user_clone.clone();
                    let candidate_clone = candidate.to_string();
                    
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(e) = websocket_clone.send_signaling_message(&target_clone, "ice-candidate", &candidate_clone).await {
                            console::error_2(&"Failed to send ICE candidate:".into(), &e);
                        } else {
                            console::log_1(&"✅ ICE candidate sent successfully".into());
                        }
                    });
                } else {
                    console::error_1(&"❌ WebSocket not available for ICE candidate transmission".into());
                }
            });
            
            console::log_1(&"✅ ICE candidate automatic transmission enabled".into());
        } else {
            console::error_1(&"❌ WebRTC client not available".into());
        }
        
        Ok(())
    }
}