use crate::types::{Message, MessageType};
use crate::video_chat::VideoChat;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, BinaryType, CloseEvent, ErrorEvent, MessageEvent, WebSocket};

#[derive(Clone)]
pub struct WebSocketClient {
    ws: WebSocket,
    username: String,
    video_chat: VideoChat,
}

impl WebSocketClient {
    pub async fn new(url: &str, username: &str, video_chat: VideoChat) -> Result<Self, JsValue> {
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        let client = WebSocketClient {
            ws,
            username: username.to_string(),
            video_chat,
        };

        client.setup_event_handlers()?;

        client.wait_for_connection_and_register().await?;

        Ok(client)
    }

    pub async fn wait_for_connection_and_register(&self) -> Result<(), JsValue> {
        console::log_1(&"Waiting for WebSocket connection...".into());

        let (tx, rx) = futures::channel::oneshot::channel();
        let tx = Rc::new(RefCell::new(Some(tx)));

        let onopen_callback = Closure::wrap(Box::new(move || {
            if let Some(tx) = tx.borrow_mut().take() {
                if tx.send(()).is_err() {
                    console::error_1(&"Failed to send open signal".into());
                }
            }
        }) as Box<dyn FnMut()>);

        self.ws
            .set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        if let Err(_) = rx.await {
            return Err(JsValue::from_str("WebSocket connection failed to open."));
        }

        console::log_1(&"WebSocket is open, sending registration...".into());

        let registration_msg = Message::new_text(
            Some(self.username.clone()),
            None,
            format!("User {} connected", self.username),
        );

        self.send_message(&registration_msg).await?;
        console::log_1(&"Username registration completed".into());

        Ok(())
    }

    fn setup_event_handlers(&self) -> Result<(), JsValue> {
        let video_chat_clone = self.video_chat.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
                let message_str = text.as_string().unwrap_or_default();
                match serde_json::from_str::<Message>(&message_str) {
                    Ok(message) => {
                        let vc_clone = video_chat_clone.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let sender_id_opt = message.sender_id.clone();
                            let sender_username_opt = message.sender_username.clone();
                            let sender_display = sender_username_opt
                                .clone()
                                .unwrap_or_else(|| {
                                    sender_id_opt
                                        .clone()
                                        .unwrap_or_else(|| "system".to_string())
                                });
                            let sender_routing = sender_username_opt
                                .clone()
                                .unwrap_or_else(|| sender_id_opt.clone().unwrap_or_default());

                            match message.message_type {
                                MessageType::TextChat {
                                    target_user_id,
                                    content,
                                } => {
                                    let is_broadcast = target_user_id.is_none();
                                    let _ = vc_clone
                                        .handle_message(&sender_display, &content, is_broadcast);
                                }
                                MessageType::WebRTCSignaling { signaling_data, .. } => {
                                    let sig_type = signaling_data
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    let data = match sig_type.as_str() {
                                        "offer" | "answer" => signaling_data
                                            .get("sdp")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        "ice-candidate" => signaling_data
                                            .get("candidate")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        _ => signaling_data
                                            .get("data")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    };

                                    if !sender_routing.is_empty() && !sig_type.is_empty() {
                                        if let Err(e) = vc_clone
                                            .handle_signaling_message(
                                                &sender_routing,
                                                &sig_type,
                                                &data,
                                            )
                                            .await
                                        {
                                            console::error_2(
                                                &"Error handling signaling message:".into(),
                                                &e,
                                            );
                                        }
                                    } else {
                                        console::warn_1(
                                            &"Received signaling message without sender or type"
                                                .into(),
                                        );
                                    }
                                }
                                MessageType::GenericMessage { content, .. } => {
                                    let _ = vc_clone.handle_message(&sender_display, &content, false);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        console::error_2(&"Failed to parse message:".into(), &e.to_string().into());
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        self.ws
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        let onerror_callback = Closure::wrap(Box::new(move |error: ErrorEvent| {
            console::error_2(&"WebSocket error:".into(), &error.message().into());
        }) as Box<dyn FnMut(ErrorEvent)>);
        self.ws
            .set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        let onclose_callback = Closure::wrap(Box::new(move |event: CloseEvent| {
            console::log_2(&"WebSocket closed with code:".into(), &event.code().into());
        }) as Box<dyn FnMut(CloseEvent)>);
        self.ws
            .set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        Ok(())
    }

    pub async fn send_broadcast_message(&self, content: &str) -> Result<(), JsValue> {
        let msg = Message::new_text(Some(self.username.clone()), None, content.to_string());
        self.send_message(&msg).await
    }

    pub async fn send_direct_message(&self, target: &str, content: &str) -> Result<(), JsValue> {
        let msg = Message::new_text(
            Some(self.username.clone()),
            Some(target.to_string()),
            content.to_string(),
        );
        self.send_message(&msg).await
    }

    pub async fn send_signaling_message(
        &self,
        target: &str,
        signaling_type: &str,
        data: &str,
    ) -> Result<(), JsValue> {
        let signaling_data = match signaling_type {
            "offer" | "answer" => serde_json::json!({ "type": signaling_type, "sdp": data }),
            "ice-candidate" => serde_json::json!({ "type": "ice-candidate", "candidate": data }),
            _ => serde_json::json!({ "type": signaling_type, "data": data }),
        };
        let msg = Message::new_webrtc(
            Some(self.username.clone()),
            target.to_string(),
            signaling_data,
        );
        self.send_message(&msg).await
    }

    async fn send_message(&self, message: &Message) -> Result<(), JsValue> {
        let json_str = serde_json::to_string(message).map_err(|e| JsValue::from(e.to_string()))?;

        if self.ws.ready_state() != 1 {
            // OPEN
            return Err(JsValue::from_str("WebSocket is not open."));
        }

        self.ws.send_with_str(&json_str)
    }

    pub fn close(&self) -> Result<(), JsValue> {
        self.ws.close()
    }
}
