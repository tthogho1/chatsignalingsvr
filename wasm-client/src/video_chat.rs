use wasm_bindgen::prelude::*;
use web_sys::{console, MediaStream};
use std::rc::Rc;
use std::cell::RefCell;

use crate::websocket_client::WebSocketClient;
use crate::webrtc_client::WebRTCClient;
use crate::dom_helpers::DomHelpers;

// WASM client entry point
#[wasm_bindgen]
pub struct VideoChat {
    websocket: Rc<RefCell<Option<WebSocketClient>>>,
    webrtc: Rc<RefCell<Option<WebRTCClient>>>,
    dom: DomHelpers,
    username: Option<String>,
    current_call: Option<String>,
    is_connected: bool,
    is_in_call: bool,
}

#[wasm_bindgen]
impl VideoChat {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<VideoChat, JsValue> {
        console_error_panic_hook::set_once();
        
        Ok(VideoChat {
            websocket: Rc::new(RefCell::new(None)),
            webrtc: Rc::new(RefCell::new(None)),
            dom: DomHelpers::new()?,
            username: None,
            current_call: None,
            is_connected: false,
            is_in_call: false,
        })
    }

    #[wasm_bindgen]
    pub async fn initialize(&mut self) -> Result<(), JsValue> {
        console::log_1(&"Initializing VideoChat WASM client...".into());
        
        // Initialize WebRTC client
        let webrtc_client = WebRTCClient::new().await?;
        *self.webrtc.borrow_mut() = Some(webrtc_client);
        
        // Setup DOM event listeners
        self.dom.setup_event_listeners(self)?;
        
        console::log_1(&"VideoChat WASM client initialized successfully".into());
        Ok(())
    }

    #[wasm_bindgen]
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

    #[wasm_bindgen]
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

    #[wasm_bindgen]
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
        let webrtc_client = WebRTCClient::new().await?;
        *self.webrtc.borrow_mut() = Some(webrtc_client);
        
        console::log_1(&"Getting user media...".into());
        
        // Get user media
        if let Some(ref webrtc) = &*self.webrtc.borrow() {
            let local_stream = webrtc.get_user_media().await?;
            console::log_1(&"User media obtained successfully".into());
            self.dom.set_local_video_stream(&local_stream)?;
            
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

    #[wasm_bindgen]
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

    #[wasm_bindgen]
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

    #[wasm_bindgen]
    pub async fn toggle_camera(&mut self) -> Result<(), JsValue> {
        if let Some(ref mut webrtc) = &mut *self.webrtc.borrow_mut() {
            let enabled = webrtc.toggle_camera().await?;
            self.dom.update_camera_button(enabled)?;
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn toggle_microphone(&mut self) -> Result<(), JsValue> {
        if let Some(ref mut webrtc) = &mut *self.webrtc.borrow_mut() {
            let enabled = webrtc.toggle_microphone().await?;
            self.dom.update_microphone_button(enabled)?;
        }
        Ok(())
    }

    // Internal methods
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

    #[wasm_bindgen]
    pub async fn handle_signaling_message(&mut self, from: &str, signaling_type: &str, data: &str) -> Result<(), JsValue> {
        // WebRTCシグナリング処理の詳細ログ
        console::group_1(&"=== Handling WebRTC Signaling ===".into());
        console::log_2(&"From:".into(), &from.into());
        console::log_2(&"Type:".into(), &signaling_type.into());
        console::log_2(&"Data length:".into(), &data.len().into());
        console::log_2(&"Current user:".into(), &self.username.as_ref().unwrap_or(&"unknown".to_string()).into());
        
        match signaling_type {
            "offer" => {
                console::log_1(&"Processing incoming call offer".into());
                console::log_2(&"SDP preview (100 chars):".into(), &data.chars().take(100).collect::<String>().into());
                
                // Handle incoming call
                let should_accept = self.dom.show_incoming_call_dialog(from)?;
                if should_accept {
                    console::log_1(&"Call accepted, setting up WebRTC connection".into());
                    if let Some(ref webrtc) = &*self.webrtc.borrow() {
                        let local_stream = webrtc.get_user_media().await?;
                        console::log_1(&"Local stream obtained".into());
                        self.dom.set_local_video_stream(&local_stream)?;
                        
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
                    console::log_1(&"Call rejected by user".into());
                    // Reject call
                    if let Some(ref websocket) = &*self.websocket.borrow() {
                        websocket.send_signaling_message(from, "reject", "").await?;
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

    #[wasm_bindgen]
    pub fn handle_message(&self, from: &str, message: &str, is_broadcast: bool) -> Result<(), JsValue> {
        let message_type = if is_broadcast { "broadcast" } else { "direct" };
        self.dom.add_chat_message(from, message, message_type)?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_remote_stream(&self, stream: &MediaStream) -> Result<(), JsValue> {
        self.dom.set_remote_video_stream(stream)?;
        Ok(())
    }
}