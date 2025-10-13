use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    window, Document, Element, HtmlElement, HtmlVideoElement,
    HtmlButtonElement, HtmlInputElement, MediaStream
};

#[derive(Clone)]
pub struct DomHelpers {
    document: Document,
}

impl DomHelpers {
    pub fn new() -> Result<Self, JsValue> {
        let window = window().ok_or("No global window exists")?;
        let document = window.document().ok_or("No document found")?;
        
        Ok(DomHelpers { document })
    }

    pub fn setup_event_listeners(&self, _video_chat: &crate::video_chat::VideoChat) -> Result<(), JsValue> {
        // For now, we'll handle events manually in JavaScript
        Ok(())
    }

    pub fn get_element_by_id(&self, id: &str) -> Result<Element, JsValue> {
        self.document
            .get_element_by_id(id)
            .ok_or_else(|| JsValue::from_str(&format!("Element with id '{}' not found", id)))
    }

    pub fn get_input_value(id: &str) -> Result<String, JsValue> {
        let window = window().ok_or("No global window exists")?;
        let document = window.document().ok_or("No document found")?;
        let element = document
            .get_element_by_id(id)
            .ok_or_else(|| JsValue::from_str(&format!("Element with id '{}' not found", id)))?;
        let input: HtmlInputElement = element.dyn_into()?;
        Ok(input.value())
    }

    pub fn set_local_video_stream(&self, stream: &MediaStream) -> Result<(), JsValue> {
        if let Ok(video) = self.get_element_by_id("local-video") {
            let video_element: HtmlVideoElement = video.dyn_into()?;
            video_element.set_src_object(Some(stream));
        }
        Ok(())
    }

    pub fn set_remote_video_stream(&self, stream: &MediaStream) -> Result<(), JsValue> {
        if let Ok(video) = self.get_element_by_id("remote-video") {
            let video_element: HtmlVideoElement = video.dyn_into()?;
            video_element.set_src_object(Some(stream));
        }
        Ok(())
    }

    pub fn clear_remote_video(&self) -> Result<(), JsValue> {
        if let Ok(video) = self.get_element_by_id("remote-video") {
            let video_element: HtmlVideoElement = video.dyn_into()?;
            video_element.set_src_object(None);
        }
        Ok(())
    }

    pub fn update_connection_status(&self, connected: bool, username: &str) -> Result<(), JsValue> {
        // Show/hide connection panel
        if let Ok(connection_panel) = self.get_element_by_id("connection-panel") {
            let panel: HtmlElement = connection_panel.dyn_into()?;
            let display = if connected { "none" } else { "block" };
            panel.set_attribute("style", &format!("display: {};", display))?;
        }

        // Show/hide main content
        if let Ok(main_content) = self.get_element_by_id("main-content") {
            let content: HtmlElement = main_content.dyn_into()?;
            let display = if connected { "block" } else { "none" };
            content.set_attribute("style", &format!("display: {};", display))?;
        }

        // Update username display
        if connected {
            if let Ok(username_display) = self.get_element_by_id("username-display") {
                username_display.set_text_content(Some(username));
            }
        }

        Ok(())
    }

    pub fn update_call_status(&self, in_call: bool, call_target: &str) -> Result<(), JsValue> {
        // Update call control buttons
        if let Ok(start_call_btn) = self.get_element_by_id("start-call") {
            let btn: HtmlButtonElement = start_call_btn.dyn_into()?;
            btn.set_disabled(in_call);
        }

        if let Ok(end_call_btn) = self.get_element_by_id("end-call") {
            let btn: HtmlButtonElement = end_call_btn.dyn_into()?;
            btn.set_disabled(!in_call);
        }

        // Update call status display
        if let Ok(call_status) = self.get_element_by_id("call-status") {
            let status_text = if in_call {
                format!("In call with: {}", call_target)
            } else {
                "Not in call".to_string()
            };
            call_status.set_text_content(Some(&status_text));
        }

        Ok(())
    }

    pub fn update_camera_button(&self, enabled: bool) -> Result<(), JsValue> {
        if let Ok(btn) = self.get_element_by_id("toggle-camera") {
            let button: HtmlButtonElement = btn.dyn_into()?;
            button.set_text_content(Some(if enabled { "📹" } else { "📹❌" }));
        }
        Ok(())
    }

    pub fn update_microphone_button(&self, enabled: bool) -> Result<(), JsValue> {
        if let Ok(btn) = self.get_element_by_id("toggle-mic") {
            let button: HtmlButtonElement = btn.dyn_into()?;
            button.set_text_content(Some(if enabled { "🎤" } else { "🎤❌" }));
        }
        Ok(())
    }

    pub fn show_incoming_call_dialog(&self, from: &str) -> Result<bool, JsValue> {
        let window = window().ok_or("No global window exists")?;
        let message = format!("Incoming call from {}. Accept?", from);
        Ok(window.confirm_with_message(&message)?)
    }

    pub fn show_notification(&self, message: &str, _notification_type: &str) -> Result<(), JsValue> {
        // Simple alert for now - in a real implementation, this would show a proper notification
        let window = window().ok_or("No global window exists")?;
        window.alert_with_message(message)?;
        Ok(())
    }

    pub fn add_chat_message(&self, from: &str, message: &str, message_type: &str) -> Result<(), JsValue> {
        if let Ok(chat_messages) = self.get_element_by_id("chat-messages") {
            // Create message element
            let message_element = self.document.create_element("div")?;
            message_element.set_class_name("chat-message");
            
            // Format timestamp
            let timestamp = js_sys::Date::new_0();
            let time_string = timestamp.to_locale_time_string("en-US");
            
            // Create formatted message
            let formatted_message = format!(
                "[{}] {}: {}",
                time_string.as_string().unwrap_or_default(),
                from,
                message
            );
            
            message_element.set_text_content(Some(&formatted_message));
            
            // Add styling based on message type
            match message_type {
                "broadcast" => {
                    message_element.set_class_name("chat-message broadcast-message");
                }
                "direct" => {
                    message_element.set_class_name("chat-message direct-message");
                }
                _ => {
                    // Default styling
                }
            }
            
            // Append to chat container and auto-scroll
            let container: HtmlElement = chat_messages.dyn_into()?;
            container.append_child(&message_element)?;
            container.set_scroll_top(container.scroll_height());
        }
        Ok(())
    }
}