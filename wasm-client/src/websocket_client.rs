use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, CloseEvent, ErrorEvent, BinaryType};
use serde_json;
use crate::types::*;
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
                
                if let Ok(message) = serde_json::from_str::<ChatMessage>(&message_str) {
                    let mut video_chat_ref = video_chat_clone.borrow_mut();
                    
                    match message.message_type.as_str() {
                        "broadcast" => {
                            let _ = video_chat_ref.handle_message(&message.sender, &message.content, true);
                        }
                        "direct" => {
                            let _ = video_chat_ref.handle_message(&message.sender, &message.content, false);
                        }
                        "signaling" => {
                            if let (Some(signaling_type), Some(data)) = (&message.signaling_type, &message.data) {
                                // Clone only the data needed into locals to avoid moving the Rc
                                let sender = message.sender.clone();
                                let signaling_type = signaling_type.clone();
                                let data = data.clone();
                                let video_chat_async = video_chat_clone.clone();
                                wasm_bindgen_futures::spawn_local(async move {
                                    let mut vc = video_chat_async.borrow_mut();
                                    let _ = vc.handle_signaling_message(&sender, &signaling_type, &data).await;
                                });
                            }
                        }
                        _ => {
                            web_sys::console::log_2(&"Unknown message type:".into(), &message.message_type.into());
                        }
                    }
                } else {
                    web_sys::console::log_2(&"Failed to parse message:".into(), &message_str.into());
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
        let message = ChatMessage {
            message_type: "broadcast".to_string(),
            content: content.to_string(),
            sender: self.username.clone(),
            target: None,
            signaling_type: None,
            data: None,
        };

        self.send_message(&message).await
    }

    pub async fn send_direct_message(&self, target: &str, content: &str) -> Result<(), JsValue> {
        let message = ChatMessage {
            message_type: "direct".to_string(),
            content: content.to_string(),
            sender: self.username.clone(),
            target: Some(target.to_string()),
            signaling_type: None,
            data: None,
        };

        self.send_message(&message).await
    }

    pub async fn send_signaling_message(&self, target: &str, signaling_type: &str, data: &str) -> Result<(), JsValue> {
        let message = ChatMessage {
            message_type: "signaling".to_string(),
            content: "".to_string(),
            sender: self.username.clone(),
            target: Some(target.to_string()),
            signaling_type: Some(signaling_type.to_string()),
            data: Some(data.to_string()),
        };

        self.send_message(&message).await
    }

    async fn send_message(&self, message: &ChatMessage) -> Result<(), JsValue> {
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