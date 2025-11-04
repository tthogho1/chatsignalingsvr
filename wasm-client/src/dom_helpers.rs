use crate::video_chat::VideoChat;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use js_sys::Reflect;
use web_sys::{
    console, window, Document, Element, HtmlButtonElement, HtmlElement, HtmlInputElement,
    HtmlVideoElement, MediaStream, Node,
};

pub struct DomHelpers;

impl DomHelpers {
    fn get_document() -> Result<Document, JsValue> {
        let window = window().ok_or_else(|| JsValue::from_str("No global window exists"))?;
        window
            .document()
            .ok_or_else(|| JsValue::from_str("No document found"))
    }

    pub fn setup_event_listeners(video_chat: VideoChat) -> Result<(), JsValue> {
        // The new HTML front-end wires up events directly in JavaScript. We only
        // attach handlers here when the legacy markup (without the `-btn`
        // suffixes) is present. This prevents double bindings in the new UI.
        let legacy_markup_present = Self::get_element_by_id("connect").is_ok();
        if !legacy_markup_present {
            console::log_1(&"Skipping WASM auto-wiring for DOM events; host page manages bindings.".into());
            return Ok(());
        }

        Self::setup_connect_listener(video_chat.clone())?;
        Self::setup_disconnect_listener(video_chat.clone())?;
        Self::setup_start_call_listener(video_chat.clone())?;
        Self::setup_end_call_listener(video_chat.clone())?;
        Self::setup_send_message_listener(video_chat.clone())?;
        Self::setup_toggle_camera_listener(video_chat.clone())?;
        Self::setup_toggle_mic_listener(video_chat)?;
        Ok(())
    }

    fn setup_connect_listener(video_chat: VideoChat) -> Result<(), JsValue> {
        let connect_btn = Self::get_element_by_id("connect")?;
        let callback = Closure::wrap(Box::new(move || {
            let vc = video_chat.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = Self::get_input_value("server-url").unwrap_or_default();
                let username = Self::get_input_value("username").unwrap_or_default();
                if !url.is_empty() && !username.is_empty() {
                    let _ = vc.connect(&url, &username).await;
                }
            });
        }) as Box<dyn FnMut()>);
        connect_btn.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
        Ok(())
    }

    fn setup_disconnect_listener(video_chat: VideoChat) -> Result<(), JsValue> {
        let disconnect_btn = Self::get_element_by_id("disconnect")?;
        let callback = Closure::wrap(Box::new(move || {
            let _ = video_chat.disconnect();
        }) as Box<dyn FnMut()>);
        disconnect_btn
            .add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
        Ok(())
    }

    fn setup_start_call_listener(video_chat: VideoChat) -> Result<(), JsValue> {
        let start_call_btn = Self::get_element_by_id("start-call")?;
        let callback = Closure::wrap(Box::new(move || {
            let vc = video_chat.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let target = Self::get_input_value("target-user").unwrap_or_default();
                if !target.is_empty() {
                    let _ = vc.start_call(&target).await;
                }
            });
        }) as Box<dyn FnMut()>);
        start_call_btn
            .add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
        Ok(())
    }

    fn setup_end_call_listener(video_chat: VideoChat) -> Result<(), JsValue> {
        let end_call_btn = Self::get_element_by_id("end-call")?;
        let callback = Closure::wrap(Box::new(move || {
            let _ = video_chat.end_call();
        }) as Box<dyn FnMut()>);
        end_call_btn
            .add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
        Ok(())
    }

    fn setup_send_message_listener(video_chat: VideoChat) -> Result<(), JsValue> {
        let send_btn = Self::get_element_by_id("send-message")?;
        let callback = Closure::wrap(Box::new(move || {
            let vc = video_chat.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let message = Self::get_input_value("chat-input").unwrap_or_default();
                if !message.is_empty() {
                    let _ = vc.send_message(&message, None).await;
                    // Clear input after sending
                    if let Ok(input) = Self::get_input_element_by_id("chat-input") {
                        input.set_value("");
                    }
                }
            });
        }) as Box<dyn FnMut()>);
        send_btn.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
        Ok(())
    }

    fn setup_toggle_camera_listener(video_chat: VideoChat) -> Result<(), JsValue> {
        let toggle_camera_btn = match Self::get_element_by_id("toggle-camera") {
            Ok(elem) => elem,
            Err(_) => return Ok(()),
        };
        let callback = Closure::wrap(Box::new(move || {
            let vc = video_chat.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = vc.toggle_camera().await;
            });
        }) as Box<dyn FnMut()>);
        toggle_camera_btn
            .add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
        Ok(())
    }

    fn setup_toggle_mic_listener(video_chat: VideoChat) -> Result<(), JsValue> {
        let toggle_mic_btn = match Self::get_element_by_id("toggle-mic") {
            Ok(elem) => elem,
            Err(_) => return Ok(()),
        };
        let callback = Closure::wrap(Box::new(move || {
            let vc = video_chat.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = vc.toggle_microphone().await;
            });
        }) as Box<dyn FnMut()>);
        toggle_mic_btn
            .add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
        Ok(())
    }

    pub fn get_element_by_id(id: &str) -> Result<Element, JsValue> {
        let doc = Self::get_document()?;
        doc.get_element_by_id(id)
            .ok_or_else(|| JsValue::from_str(&format!("Element with id '{}' not found", id)))
    }

    pub fn get_input_element_by_id(id: &str) -> Result<HtmlInputElement, JsValue> {
        Self::get_element_by_id(id)?
            .dyn_into::<HtmlInputElement>()
            .map_err(|_| JsValue::from_str(&format!("Element '{}' is not an HtmlInputElement", id)))
    }

    pub fn get_input_value(id: &str) -> Result<String, JsValue> {
        Ok(Self::get_input_element_by_id(id)?.value())
    }

    pub fn set_local_video_stream(stream: &MediaStream) -> Result<(), JsValue> {
        if let Ok(video) = Self::get_element_by_id("local-video") {
            let video_element: HtmlVideoElement = video.dyn_into()?;
            video_element.set_src_object(Some(stream));
            Self::attempt_video_play(&video_element, "local-video");
        }
        Ok(())
    }

    pub fn set_remote_video_stream(stream: &MediaStream) -> Result<(), JsValue> {
        if let Ok(video) = Self::get_element_by_id("remote-video") {
            let video_element: HtmlVideoElement = video.dyn_into()?;
            video_element.set_src_object(Some(stream));
            Self::attempt_video_play(&video_element, "remote-video");
        }
        Ok(())
    }

    pub fn clear_remote_video() -> Result<(), JsValue> {
        if let Ok(video) = Self::get_element_by_id("remote-video") {
            let video_element: HtmlVideoElement = video.dyn_into()?;
            video_element.set_src_object(None);
        }
        Ok(())
    }

    pub fn update_connection_status(is_connected: bool, username: &str) -> Result<(), JsValue> {
        if let Ok(status_el) = Self::get_element_by_id("connection-status") {
            let status_el: HtmlElement = status_el.dyn_into()?;
            if is_connected {
                status_el.set_inner_text(&format!("Connected as: {}", username));
                status_el.style().set_property("color", "green")?;
            } else {
                status_el.set_inner_text("Disconnected");
                status_el.style().set_property("color", "red")?;
            }
        }

        if let Ok(username_el) = Self::get_element_by_id("username-display") {
            let username_el: HtmlElement = username_el.dyn_into()?;
            username_el.set_inner_text(if is_connected { username } else { "" });
        }

        if let Ok(connect_panel) = Self::get_element_by_id("connection-panel") {
            let connect_panel: HtmlElement = connect_panel.dyn_into()?;
            connect_panel
                .style()
                .set_property("display", if is_connected { "none" } else { "block" })?;
        }

        if let Ok(main_content) = Self::get_element_by_id("main-content") {
            let main_content: HtmlElement = main_content.dyn_into()?;
            main_content
                .style()
                .set_property("display", if is_connected { "grid" } else { "none" })?;
        }

        if let Ok(connect_btn) = Self::get_element_by_id("connect-btn") {
            let connect_btn: HtmlButtonElement = connect_btn.dyn_into()?;
            connect_btn.set_disabled(is_connected);
            connect_btn.set_inner_text(if is_connected { "接続済み" } else { "接続" });
        }

        if let Ok(disconnect_btn) = Self::get_element_by_id("disconnect-btn") {
            let disconnect_btn: HtmlButtonElement = disconnect_btn.dyn_into()?;
            disconnect_btn
                .style()
                .set_property("display", if is_connected { "inline-block" } else { "none" })?;
        }

        Ok(())
    }

    pub fn update_call_status(is_in_call: bool, target_user: &str) -> Result<(), JsValue> {
        if let Ok(status_el) = Self::get_element_by_id("call-status") {
            let status_el: HtmlElement = status_el.dyn_into()?;
            if is_in_call {
                status_el.set_inner_text(&format!("In call with: {}", target_user));
                status_el.style().set_property("display", "block")?;
            } else {
                status_el.style().set_property("display", "none")?;
            }
        }

        if let Ok(start_btn) = Self::get_element_by_id("start-call-btn") {
            let start_btn: HtmlButtonElement = start_btn.dyn_into()?;
            start_btn
                .style()
                .set_property("display", if is_in_call { "none" } else { "inline-block" })?;
            start_btn.set_disabled(is_in_call);
        }

        if let Ok(end_btn) = Self::get_element_by_id("end-call-btn") {
            let end_btn: HtmlButtonElement = end_btn.dyn_into()?;
            end_btn
                .style()
                .set_property("display", if is_in_call { "inline-block" } else { "none" })?;
            end_btn.set_disabled(!is_in_call);
        }

        Ok(())
    }

    pub fn add_chat_message(from: &str, message: &str, msg_type: &str) -> Result<(), JsValue> {
        if let Ok(chat_box) = Self::get_element_by_id("chat-box") {
            let doc = Self::get_document()?;
            let p = doc.create_element("p")?;
            p.set_inner_html(&format!("<strong>{}:</strong> {}", from, message));
            p.set_class_name(msg_type);
            chat_box.append_child(&p)?;
            chat_box.set_scroll_top(chat_box.scroll_height());
        } else {
            let level = if msg_type == "broadcast" { "info" } else { "chat" };
            let composed = format!("{}: {}", from, message);
            Self::show_notification(&composed, level.as_ref())?;
        }
        Ok(())
    }

    pub fn show_incoming_call_dialog(from: &str) -> Result<bool, JsValue> {
        let window = window().ok_or_else(|| JsValue::from_str("No global window exists"))?;
        let result =
            window.confirm_with_message(&format!("Incoming call from {}. Answer?", from))?;
        Ok(result)
    }

    pub fn show_notification(message: &str, level: &str) -> Result<(), JsValue> {
        if let Ok(container) = Self::get_element_by_id("notifications") {
            let doc = Self::get_document()?;
            let notification = doc.create_element("div")?.dyn_into::<HtmlElement>()?;
            notification.set_class_name("notification");
            notification.set_inner_text(message);

            let border_color = match level {
                "error" => "#ff4757",
                "warning" => "#ffa502",
                "success" => "#2ed573",
                _ => "#667eea",
            };

            notification
                .style()
                .set_property("border-left-color", border_color)?;

            container.append_child(&notification)?;

            let window = window().ok_or_else(|| JsValue::from_str("No global window exists"))?;
            let element = notification.clone();
            let closure = Closure::wrap(Box::new(move || {
                if let Some(parent) = element.parent_node() {
                    let node: web_sys::Node = element.clone().into();
                    let _ = parent.remove_child(&node);
                }
            }) as Box<dyn FnMut()>);

            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    5000,
                )?;
            closure.forget();
        } else {
            let window = window().ok_or_else(|| JsValue::from_str("No global window exists"))?;
            window.alert_with_message(&format!("[{}] {}", level.to_uppercase(), message))?;
        }
        Ok(())
    }

    pub fn update_camera_button(is_on: bool) -> Result<(), JsValue> {
        let btn = Self::get_element_by_id("toggle-camera")?.dyn_into::<HtmlButtonElement>()?;
        btn.set_inner_text(if is_on {
            "Turn Camera Off"
        } else {
            "Turn Camera On"
        });
        Ok(())
    }

    pub fn update_microphone_button(is_on: bool) -> Result<(), JsValue> {
        let btn = Self::get_element_by_id("toggle-mic")?.dyn_into::<HtmlButtonElement>()?;
        btn.set_inner_text(if is_on { "Mute Mic" } else { "Unmute Mic" });
        Ok(())
    }
}

impl DomHelpers {
    fn attempt_video_play(video: &HtmlVideoElement, label: &str) {
        match video.play() {
            Ok(promise) => {
                let label = label.to_string();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(err) = JsFuture::from(promise).await {
                        let err_name = Reflect::get(&err, &JsValue::from_str("name"))
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();

                        let prefix = if err_name == "AbortError" {
                            JsValue::from_str(&format!(
                                "{} play aborted (benign, likely due to source change)",
                                label
                            ))
                        } else {
                            JsValue::from_str(&format!("{} play promise rejected", label))
                        };

                        if err_name == "AbortError" {
                            console::debug_2(&prefix, &err);
                        } else {
                            console::warn_2(&prefix, &err);
                        }
                    }
                });
            }
            Err(err) => {
                let prefix = JsValue::from_str(&format!("{} play invocation failed", label));
                console::warn_2(&prefix, &err);
            }
        }
    }
}
