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

        console::log_2(&"Starting call to".into(), &target_user.into());
        
        // Get user media
        if let Some(ref webrtc) = &*self.webrtc.borrow() {
            let local_stream = webrtc.get_user_media().await?;
            self.dom.set_local_video_stream(&local_stream)?;
            
            // Create offer (SDP)
            let offer = webrtc.create_offer().await?;
            
            // Send offer through WebSocket
            if let Some(ref websocket) = &*self.websocket.borrow() {
                websocket.send_signaling_message(target_user, "offer", &offer).await?;
            }
        }
        
        self.current_call = Some(target_user.to_string());
        self.is_in_call = true;
        
        // Update UI
        self.dom.update_call_status(true, target_user)?;
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn end_call(&mut self) -> Result<(), JsValue> {
        console::log_1(&"Ending call".into());
        
        // Close peer connection
        if let Some(ref webrtc) = &*self.webrtc.borrow() {
            webrtc.close_connection()?;
        }
        
        // Notify other party
        if let Some(ref current_call) = &self.current_call {
            if let Some(ref websocket) = &*self.websocket.borrow() {
                let _ = websocket.send_signaling_message(current_call, "hangup", "");
            }
        }
        
        self.current_call = None;
        self.is_in_call = false;
        
        // Update UI
        self.dom.update_call_status(false, "")?;
        self.dom.clear_remote_video()?;
        
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
        console::log_3(&"Received signaling message".into(), &signaling_type.into(), &from.into());
        
        match signaling_type {
            "offer" => {
                // Handle incoming call
                let should_accept = self.dom.show_incoming_call_dialog(from)?;
                if should_accept {
                    if let Some(ref webrtc) = &*self.webrtc.borrow() {
                        let local_stream = webrtc.get_user_media().await?;
                        self.dom.set_local_video_stream(&local_stream)?;
                        
                        let answer = webrtc.handle_offer(data).await?;
                        
                        if let Some(ref websocket) = &*self.websocket.borrow() {
                            websocket.send_signaling_message(from, "answer", &answer).await?;
                        }
                        
                        self.current_call = Some(from.to_string());
                        self.is_in_call = true;
                        self.dom.update_call_status(true, from)?;
                    }
                } else {
                    // Reject call
                    if let Some(ref websocket) = &*self.websocket.borrow() {
                        websocket.send_signaling_message(from, "reject", "").await?;
                    }
                }
            }
            "answer" => {
                if let Some(ref webrtc) = &*self.webrtc.borrow() {
                    webrtc.handle_answer(data).await?;
                }
            }
            "ice-candidate" => {
                if let Some(ref webrtc) = &*self.webrtc.borrow() {
                    webrtc.handle_ice_candidate(data).await?;
                }
            }
            "hangup" => {
                self.end_call()?;
            }
            "reject" => {
                self.dom.show_notification("Call rejected", "warning")?;
                self.end_call()?;
            }
            _ => {
                console::log_2(&"Unknown signaling type:".into(), &signaling_type.into());
            }
        }
        
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