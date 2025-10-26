use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, CloseEvent, ErrorEvent, BinaryType};
use serde_json;
use crate::types::{Message, MessageType};
use crate::video_chat::VideoChat;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone)]
pub struct WebSocketClient {
    ws: WebSocket,
    username: String,
    video_chat: Rc<RefCell<VideoChat>>,
}

impl WebSocketClient {
    pub async fn new(url: &str, username: &str, video_chat: VideoChat) -> Result<Self, JsValue> {
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        
        let client = WebSocketClient {
            ws,
            username: username.to_string(),
            video_chat: Rc::new(RefCell::new(video_chat)),
        };
        
        client.setup_event_handlers()?;
        
        // Wait for WebSocket to be ready, then send registration message
        client.wait_for_connection_and_register().await?;
        
        Ok(client)
    }

    /// Wait for WebSocket connection and then send username registration
    pub async fn wait_for_connection_and_register(&self) -> Result<(), JsValue> {
        use wasm_bindgen_futures::JsFuture;
        use js_sys::Promise;
        
        web_sys::console::log_1(&"Waiting for WebSocket connection...".into());
        
        // Simple delay to allow connection to establish
        let promise = Promise::new(&mut |resolve, _reject| {
            let window = web_sys::window().unwrap();
            let callback = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                let _ = resolve.call0(&JsValue::NULL);
            }) as Box<dyn FnMut()>);
            
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(), 
                500  // Wait 500ms for connection
            );
            callback.forget();
        });
        
        JsFuture::from(promise).await?;
        web_sys::console::log_1(&"WebSocket should be ready, sending registration...".into());
        
        // Send username registration message
        web_sys::console::log_2(&"=== Registering username with server ===".into(), &self.username.clone().into());
        
        let registration_msg = Message::new_text(
            Some(self.username.clone()), 
            None, 
            format!("User {} connected and registered username", self.username)
        );
        
        web_sys::console::log_2(&"Registration message ID:".into(), &registration_msg.id.clone().into());
        web_sys::console::log_2(&"Sender ID:".into(), &registration_msg.sender_id.as_ref().unwrap_or(&"None".to_string()).into());
        
        self.send_message(&registration_msg).await?;
        web_sys::console::log_1(&"Username registration completed successfully".into());
        
        Ok(())
    }

    fn setup_event_handlers(&self) -> Result<(), JsValue> {
        let video_chat = self.video_chat.clone();
        
        // OnOpen handler
        let onopen_callback = Closure::wrap(Box::new(move |_event| {
            web_sys::console::log_1(&"WebSocket connection opened".into());
        }) as Box<dyn FnMut(JsValue)>);
        self.ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // OnMessage handler
        let video_chat_clone = video_chat.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
                let message_str = text.as_string().unwrap_or_default();
                match serde_json::from_str::<Message>(&message_str) {
                    Ok(message) => {
                        match &message.message_type {
                            MessageType::TextChat { target_user_id, content } => {
                                let from = message.sender_id.clone().unwrap_or_else(|| "system".to_string());
                                let is_broadcast = target_user_id.is_none();
                                {
                                    let vc = video_chat_clone.borrow_mut();
                                    let _ = vc.handle_message(&from, content, is_broadcast);
                                }
                            }
                            MessageType::WebRTCSignaling { target_user_id: _, signaling_data } => {
                                let sender = message.sender_username.clone()
                                    .or_else(|| message.sender_id.clone())
                                    .unwrap_or_default();
                                let sig_type_owned = signaling_data.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                
                                // 受信したWebRTCメッセージの詳細ログ出力
                                web_sys::console::group_1(&"=== WebRTC Message RECEIVED ===".into());
                                web_sys::console::log_2(&"Sender:".into(), &sender.clone().into());
                                web_sys::console::log_2(&"Message ID:".into(), &message.id.clone().into());
                                web_sys::console::log_2(&"Timestamp:".into(), &message.timestamp.clone().into());
                                web_sys::console::log_2(&"Signaling Type:".into(), &sig_type_owned.clone().into());
                                web_sys::console::log_2(&"Full Signaling Data:".into(), &signaling_data.to_string().into());
                                
                                // SDPデータの詳細ログ（offer/answerの場合）
                                if sig_type_owned == "offer" || sig_type_owned == "answer" {
                                    if let Some(sdp) = signaling_data.get("sdp").and_then(|v| v.as_str()) {
                                        web_sys::console::log_2(&"SDP Length:".into(), &sdp.len().into());
                                        web_sys::console::log_2(&"SDP Preview (100 chars):".into(), &sdp.chars().take(100).collect::<String>().into());
                                    }
                                }
                                
                                // ICE Candidateの詳細ログ
                                if sig_type_owned == "ice-candidate" {
                                    if let Some(candidate) = signaling_data.get("candidate").and_then(|v| v.as_str()) {
                                        web_sys::console::log_2(&"ICE Candidate:".into(), &candidate.into());
                                    }
                                    if let Some(sdp_mid) = signaling_data.get("sdpMid").and_then(|v| v.as_str()) {
                                        web_sys::console::log_2(&"SDP Mid:".into(), &sdp_mid.into());
                                    }
                                    if let Some(sdp_mline_index) = signaling_data.get("sdpMLineIndex").and_then(|v| v.as_u64()) {
                                        web_sys::console::log_2(&"SDP MLine Index:".into(), &sdp_mline_index.into());
                                    }
                                }
                                web_sys::console::group_end();

                                let data_owned = match sig_type_owned.as_str() {
                                    "offer" | "answer" => signaling_data
                                        .get("sdp")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    "ice-candidate" => signaling_data.to_string(),
                                    _ => String::new(),
                                };
                                let video_chat_async = video_chat_clone.clone();
                                wasm_bindgen_futures::spawn_local(async move {
                                    // RefCellから一時的に値を取り出し、clone_handleで複製して使う
                                    let mut vc_temp = video_chat_async.borrow().clone_handle();
                                    let _ = vc_temp.handle_signaling_message(&sender, &sig_type_owned, &data_owned).await;
                                });
                            }
                            MessageType::GenericMessage { target_user_id: _, content } => {
                                let from = message.sender_id.clone().unwrap_or_else(|| "system".to_string());
                                {
                                    let vc = video_chat_clone.borrow_mut();
                                    let _ = vc.handle_message(&from, content, false);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        web_sys::console::log_2(&"Failed to parse message:".into(), &message_str.into());
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        self.ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // OnError handler
        let onerror_callback = Closure::wrap(Box::new(move |error: ErrorEvent| {
            web_sys::console::error_1(&format!("WebSocket error: {:?}", error.message()).into());
        }) as Box<dyn FnMut(ErrorEvent)>);
        self.ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        // OnClose handler
        let onclose_callback = Closure::wrap(Box::new(move |event: CloseEvent| {
            web_sys::console::log_2(&"WebSocket closed with code:".into(), &event.code().into());
        }) as Box<dyn FnMut(CloseEvent)>);
        self.ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        Ok(())
    }

    pub async fn send_broadcast_message(&self, content: &str) -> Result<(), JsValue> {
        let msg = Message::new_text(Some(self.username.clone()), None, content.to_string());
        self.send_message(&msg).await
    }

    pub async fn send_direct_message(&self, target: &str, content: &str) -> Result<(), JsValue> {
        let msg = Message::new_text(Some(self.username.clone()), Some(target.to_string()), content.to_string());
        self.send_message(&msg).await
    }

    pub async fn send_signaling_message(&self, target: &str, signaling_type: &str, data: &str) -> Result<(), JsValue> {
        let signaling_data = match signaling_type {
            "offer" => serde_json::json!({"type":"offer", "sdp": data}),
            "answer" => serde_json::json!({"type":"answer", "sdp": data}),
            "ice-candidate" => serde_json::json!({"type":"ice-candidate", "candidate": data}),
            _ => serde_json::json!({"type": signaling_type, "data": data}),
        };
        let msg = Message::new_webrtc(Some(self.username.clone()), target.to_string(), signaling_data.clone());
        
        // 送信するWebRTCメッセージの詳細ログ出力
        web_sys::console::group_1(&"=== WebRTC Message SENDING ===".into());
        web_sys::console::log_2(&"From:".into(), &self.username.clone().into());
        web_sys::console::log_2(&"To:".into(), &target.into());
        web_sys::console::log_2(&"Message ID:".into(), &msg.id.clone().into());
        web_sys::console::log_2(&"Timestamp:".into(), &msg.timestamp.clone().into());
        web_sys::console::log_2(&"Signaling Type:".into(), &signaling_type.into());
        web_sys::console::log_2(&"Full Signaling Data:".into(), &signaling_data.to_string().into());
        
        // SDPデータの詳細ログ（offer/answerの場合）
        if signaling_type == "offer" || signaling_type == "answer" {
            web_sys::console::log_2(&"SDP Length:".into(), &data.len().into());
            web_sys::console::log_2(&"SDP Preview (100 chars):".into(), &data.chars().take(100).collect::<String>().into());
        }
        
        // ICE Candidateの詳細ログ
        if signaling_type == "ice-candidate" {
            web_sys::console::log_2(&"ICE Candidate:".into(), &data.into());
        }
        web_sys::console::group_end();
        
        self.send_message(&msg).await
    }

    /// Send ICE candidate to remote peer
    pub async fn send_ice_candidate(&self, target_user: &str, candidate: &str) -> Result<(), JsValue> {
        web_sys::console::group_1(&"=== Sending ICE Candidate ===".into());
        web_sys::console::log_2(&"From:".into(), &self.username.clone().into());
        web_sys::console::log_2(&"To:".into(), &target_user.into());
        web_sys::console::log_2(&"Candidate:".into(), &candidate.into());
        
        let signaling_data = serde_json::json!({
            "type": "ice-candidate",
            "candidate": candidate
        });
        
        let msg = Message::new_webrtc(
            Some(self.username.clone()),
            target_user.to_string(),
            signaling_data.clone(),
        );
        
        web_sys::console::log_2(&"ICE Candidate Data:".into(), &format!("{}", signaling_data).into());
        web_sys::console::group_end();
        
        self.send_message(&msg).await
    }

    async fn send_message(&self, message: &Message) -> Result<(), JsValue> {
        // Detailed logging for message being sent
        web_sys::console::group_1(&"=== Sending Message to Server ===".into());
        web_sys::console::log_2(&"Message ID:".into(), &message.id.clone().into());
        web_sys::console::log_2(&"Sender ID:".into(), &message.sender_id.as_ref().unwrap_or(&"<None>".to_string()).into());
        web_sys::console::log_2(&"Message Type:".into(), &format!("{:?}", message.message_type).into());
        web_sys::console::log_2(&"Timestamp:".into(), &message.timestamp.clone().into());
        
        // Message構造体の内部構造を詳細ログ出力
        web_sys::console::log_2(&"Message Struct Debug:".into(), &format!("{:#?}", message).into());
        
        // サーバーの期待する形式に手動で変換
        let json_value = match &message.message_type {
            MessageType::TextChat { target_user_id, content } => {
                serde_json::json!({
                    "id": message.id,
                    "sender_id": message.sender_id,
                    "timestamp": message.timestamp,
                    "message_type": {
                        "TextChat": {
                            "target_user_id": target_user_id,
                            "content": content
                        }
                    }
                })
            },
            MessageType::WebRTCSignaling { target_user_id, signaling_data } => {
                serde_json::json!({
                    "id": message.id,
                    "sender_id": message.sender_id,
                    "timestamp": message.timestamp,
                    "message_type": {
                        "WebRTCSignaling": {
                            "target_user_id": target_user_id,
                            "signaling_data": signaling_data
                        }
                    }
                })
            },
            MessageType::GenericMessage { target_user_id, content } => {
                serde_json::json!({
                    "id": message.id,
                    "sender_id": message.sender_id,
                    "timestamp": message.timestamp,
                    "message_type": {
                        "GenericMessage": {
                            "target_user_id": target_user_id,
                            "content": content
                        }
                    }
                })
            }
        };
        
        let json_str = json_value.to_string();
        web_sys::console::log_2(&"JSON String Length:".into(), &json_str.len().into());
        web_sys::console::log_2(&"Manually Constructed JSON:".into(), &json_str.clone().into());
        web_sys::console::log_2(&"WebSocket Ready State:".into(), &self.ws.ready_state().into());
        
        // JSON構造の詳細確認
        if let Ok(parsed_back) = serde_json::from_str::<serde_json::Value>(&json_str) {
            web_sys::console::log_2(&"JSON Validation:".into(), &"✅ JSON is valid".into());
            web_sys::console::log_2(&"JSON Structure:".into(), &format!("{:#}", parsed_back).into());
        } else {
            web_sys::console::error_1(&"❌ Generated JSON is invalid!".into());
        }
        
        match self.ws.send_with_str(&json_str) {
            Ok(_) => {
                web_sys::console::log_1(&"✅ Message sent successfully".into());
                web_sys::console::group_end();
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_1(&format!("❌ Failed to send message: {:?}", e).into());
                web_sys::console::group_end();
                Err(e)
            }
        }
    }

    pub fn close(&self) -> Result<(), JsValue> {
        self.ws.close()?;
        Ok(())
    }
}