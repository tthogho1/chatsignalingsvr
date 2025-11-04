use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::console;

use crate::dom_helpers::DomHelpers;
use crate::webrtc_client::WebRTCClient;
use crate::websocket_client::WebSocketClient;

// The core application state, designed for single-threaded WASM environment.
#[wasm_bindgen]
#[derive(Clone)]
pub struct VideoChat {
    websocket: Rc<RefCell<Option<WebSocketClient>>>,
    webrtc: Rc<RefCell<Option<WebRTCClient>>>,
    username: Rc<RefCell<Option<String>>>,
    current_call: Rc<RefCell<Option<String>>>,
    is_connected: Rc<RefCell<bool>>,
    is_in_call: Rc<RefCell<bool>>,
}

#[wasm_bindgen]
impl VideoChat {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        VideoChat {
            websocket: Rc::new(RefCell::new(None)),
            webrtc: Rc::new(RefCell::new(None)),
            username: Rc::new(RefCell::new(None)),
            current_call: Rc::new(RefCell::new(None)),
            is_connected: Rc::new(RefCell::new(false)),
            is_in_call: Rc::new(RefCell::new(false)),
        }
    }

    pub async fn initialize(&self) -> Result<(), JsValue> {
        console::log_1(&"Initializing VideoChat WASM client...".into());
        DomHelpers::setup_event_listeners(self.clone())?;
        console::log_1(&"VideoChat WASM client initialized successfully".into());
        Ok(())
    }

    pub async fn connect(&self, url: &str, username: &str) -> Result<(), JsValue> {
        console::log_2(&"Connecting to".into(), &url.into());

        let websocket_client = WebSocketClient::new(url, username, self.clone()).await?;
        *self.websocket.borrow_mut() = Some(websocket_client);

        *self.username.borrow_mut() = Some(username.to_string());
        *self.is_connected.borrow_mut() = true;

        DomHelpers::update_connection_status(true, username)?;

        Ok(())
    }

    pub fn disconnect(&self) -> Result<(), JsValue> {
        if let Some(ref mut websocket) = *self.websocket.borrow_mut() {
            websocket.close()?;
        }
        *self.websocket.borrow_mut() = None;

        if let Some(ref webrtc) = *self.webrtc.borrow() {
            webrtc.close_connection()?;
        }
        *self.webrtc.borrow_mut() = None;

        *self.username.borrow_mut() = None;
        *self.is_connected.borrow_mut() = false;
        *self.is_in_call.borrow_mut() = false;
        *self.current_call.borrow_mut() = None;

        DomHelpers::update_connection_status(false, "")?;
        DomHelpers::update_call_status(false, "")?;
        DomHelpers::clear_remote_video()?;

        Ok(())
    }

    pub async fn start_call(&self, target_user: &str) -> Result<(), JsValue> {
        if !*self.is_connected.borrow() {
            return Err(JsValue::from_str("Not connected to server"));
        }

        console::group_1(&"=== Starting WebRTC Call ===".into());
        console::log_2(
            &"Caller:".into(),
            &self
                .username
                .borrow()
                .as_ref()
                .unwrap_or(&"unknown".to_string())
                .into(),
        );
        console::log_2(&"Target:".into(), &target_user.into());

        let self_clone_track = self.clone();
        let ontrack = Closure::wrap(Box::new(move |event: web_sys::RtcTrackEvent| {
            let streams = event.streams();
            if streams.length() > 0 {
                if let Ok(stream) = streams.get(0).dyn_into::<web_sys::MediaStream>() {
                    if let Err(e) = DomHelpers::set_remote_video_stream(&stream) {
                        console::error_2(&"Error setting remote video stream:".into(), &e);
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        let self_clone_ice = self.clone();
        let target_user_clone_ice = target_user.to_string();
        let onicecandidate =
            Closure::wrap(Box::new(move |event: web_sys::RtcPeerConnectionIceEvent| {
                if let Some(candidate) = event.candidate() {
                    let websocket_clone = self_clone_ice.websocket.clone();
                    let target_clone = target_user_clone_ice.clone();
                    let candidate_str = candidate.candidate();

                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(ws) = &*websocket_clone.borrow() {
                            if let Err(e) = ws
                                .send_signaling_message(
                                    &target_clone,
                                    "ice-candidate",
                                    &candidate_str,
                                )
                                .await
                            {
                                console::error_2(&"Failed to send ICE candidate:".into(), &e);
                            }
                        }
                    });
                }
            }) as Box<dyn FnMut(_)>);

        let webrtc_client = WebRTCClient::new(&ontrack, &onicecandidate).await?;
        ontrack.forget();
        onicecandidate.forget();

        let local_stream = webrtc_client.get_user_media().await?;
        DomHelpers::set_local_video_stream(&local_stream)?;

        let offer = webrtc_client.create_offer().await?;

        if let Some(ref websocket) = &*self.websocket.borrow() {
            websocket
                .send_signaling_message(target_user, "offer", &offer)
                .await?;
        }

        *self.webrtc.borrow_mut() = Some(webrtc_client);
        console::group_end();

        *self.current_call.borrow_mut() = Some(target_user.to_string());
        *self.is_in_call.borrow_mut() = true;

        DomHelpers::update_call_status(true, target_user)?;

        Ok(())
    }

    pub fn end_call(&self) -> Result<(), JsValue> {
        console::group_1(&"=== Ending WebRTC Call ===".into());

        if let Some(ref webrtc) = &*self.webrtc.borrow() {
            webrtc.close_connection()?;
        }

        if let Some(ref current_call) = &*self.current_call.borrow() {
            if let Some(ref websocket) = &*self.websocket.borrow() {
                let _ = websocket.send_signaling_message(current_call, "hangup", "");
            }
        }

        *self.webrtc.borrow_mut() = None;
        *self.current_call.borrow_mut() = None;
        *self.is_in_call.borrow_mut() = false;

        DomHelpers::update_call_status(false, "")?;
        DomHelpers::clear_remote_video()?;
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

    pub async fn toggle_camera(&self) -> Result<(), JsValue> {
        if let Some(ref mut webrtc) = *self.webrtc.borrow_mut() {
            let enabled = webrtc.toggle_camera().await?;
            DomHelpers::update_camera_button(enabled)?;
        }
        Ok(())
    }

    pub async fn toggle_microphone(&self) -> Result<(), JsValue> {
        if let Some(ref mut webrtc) = *self.webrtc.borrow_mut() {
            let enabled = webrtc.toggle_microphone().await?;
            DomHelpers::update_microphone_button(enabled)?;
        }
        Ok(())
    }

    pub async fn send_test_signaling_message(&self, target_user: &str) -> Result<(), JsValue> {
        if !*self.is_connected.borrow() {
            return Err(JsValue::from_str("Not connected to server"));
        }

        let websocket = self.websocket.borrow().clone();

        if let Some(websocket) = websocket {
            websocket
                .send_signaling_message(target_user, "test", "ping")
                .await?;
        } else {
            return Err(JsValue::from_str("WebSocket client is not available"));
        }

        Ok(())
    }

    pub async fn handle_signaling_message(
        &self,
        from: &str,
        signaling_type: &str,
        data: &str,
    ) -> Result<(), JsValue> {
        console::group_1(&"=== Handling WebRTC Signaling ===".into());

        match signaling_type {
            "offer" => {
                let should_accept = DomHelpers::show_incoming_call_dialog(from)?;

                if should_accept {
                    let self_clone_track = self.clone();
                    let ontrack = Closure::wrap(Box::new(move |event: web_sys::RtcTrackEvent| {
                        let streams = event.streams();
                        if streams.length() > 0 {
                            if let Ok(stream) = streams.get(0).dyn_into::<web_sys::MediaStream>() {
                                if let Err(e) = DomHelpers::set_remote_video_stream(&stream) {
                                    console::error_2(
                                        &"Error setting remote video stream:".into(),
                                        &e,
                                    );
                                }
                            }
                        }
                    }) as Box<dyn FnMut(_)>);

                    let self_clone_ice = self.clone();
                    let from_clone_ice = from.to_string();
                    let onicecandidate =
                        Closure::wrap(Box::new(move |event: web_sys::RtcPeerConnectionIceEvent| {
                            if let Some(candidate) = event.candidate() {
                                let websocket_clone = self_clone_ice.websocket.clone();
                                let target_clone = from_clone_ice.clone();
                                let candidate_str = candidate.candidate();

                                wasm_bindgen_futures::spawn_local(async move {
                                    if let Some(ws) = &*websocket_clone.borrow() {
                                        if let Err(e) = ws
                                            .send_signaling_message(
                                                &target_clone,
                                                "ice-candidate",
                                                &candidate_str,
                                            )
                                            .await
                                        {
                                            console::error_2(
                                                &"Failed to send ICE candidate:".into(),
                                                &e,
                                            );
                                        }
                                    }
                                });
                            }
                        }) as Box<dyn FnMut(_)>);

                    let webrtc_client = WebRTCClient::new(&ontrack, &onicecandidate).await?;
                    ontrack.forget();
                    onicecandidate.forget();

                    let local_stream = webrtc_client.get_user_media().await?;
                    DomHelpers::set_local_video_stream(&local_stream)?;

                    let answer = webrtc_client.handle_offer(data).await?;

                    if let Some(ref websocket) = &*self.websocket.borrow() {
                        websocket
                            .send_signaling_message(from, "answer", &answer)
                            .await?;
                    }

                    *self.webrtc.borrow_mut() = Some(webrtc_client);
                    *self.current_call.borrow_mut() = Some(from.to_string());
                    *self.is_in_call.borrow_mut() = true;
                    DomHelpers::update_call_status(true, from)?;
                } else {
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
                DomHelpers::show_notification("Call rejected", "warning")?;
                self.end_call()?;
            }
            _ => {}
        }
        console::group_end();

        Ok(())
    }

    pub fn handle_message(
        &self,
        from: &str,
        message: &str,
        is_broadcast: bool,
    ) -> Result<(), JsValue> {
        let message_type = if is_broadcast { "broadcast" } else { "direct" };
        DomHelpers::add_chat_message(from, message, message_type)?;
        Ok(())
    }
}
