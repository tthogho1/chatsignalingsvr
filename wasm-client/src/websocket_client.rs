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
        
        Ok(client)
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
                                let mut vc = video_chat_clone.borrow_mut();
                                let _ = vc.handle_message(&from, content, is_broadcast);
                            }
                            MessageType::WebRTCSignaling { target_user_id: _, signaling_data } => {
                                let sender = message.sender_id.clone().unwrap_or_default();
                                let sig_type_owned = signaling_data.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
                                    let mut vc = video_chat_async.borrow_mut();
                                    let _ = vc.handle_signaling_message(&sender, &sig_type_owned, &data_owned).await;
                                });
                            }
                            MessageType::GenericMessage { target_user_id: _, content } => {
                                let from = message.sender_id.clone().unwrap_or_else(|| "system".to_string());
                                let mut vc = video_chat_clone.borrow_mut();
                                let _ = vc.handle_message(&from, content, false);
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
        let msg = Message::new_webrtc(Some(self.username.clone()), target.to_string(), signaling_data);
        self.send_message(&msg).await
    }

    async fn send_message(&self, message: &Message) -> Result<(), JsValue> {
        match serde_json::to_string(message) {
            Ok(json_str) => {
                self.ws.send_with_str(&json_str)?;
                Ok(())
            }
            Err(e) => {
                Err(JsValue::from_str(&format!("Failed to serialize message: {}", e)))
            }
        }
    }

    pub fn close(&self) -> Result<(), JsValue> {
        self.ws.close()?;
        Ok(())
    }
}