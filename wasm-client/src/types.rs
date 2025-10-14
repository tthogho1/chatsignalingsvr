use serde::{Deserialize, Serialize};

// Mirror server-side models (src/models/message.rs)

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    TextChat {
        target_user_id: Option<String>,
        content: String,
    },
    WebRTCSignaling {
        target_user_id: String,
        signaling_data: serde_json::Value,
    },
    GenericMessage {
        target_user_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub sender_id: Option<String>,
    pub timestamp: String, // ISO 8601 string for portability
    pub message_type: MessageType,
}

impl Message {
    pub fn new_text(sender: Option<String>, target_user_id: Option<String>, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id: sender,
            timestamp: js_sys::Date::new_0().to_iso_string().into(),
            message_type: MessageType::TextChat { target_user_id, content },
        }
    }

    pub fn new_webrtc(sender: Option<String>, target_user_id: String, signaling_data: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id: sender,
            timestamp: js_sys::Date::new_0().to_iso_string().into(),
            message_type: MessageType::WebRTCSignaling { target_user_id, signaling_data },
        }
    }

    pub fn new_generic(sender: Option<String>, target_user_id: String, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id: sender,
            timestamp: js_sys::Date::new_0().to_iso_string().into(),
            message_type: MessageType::GenericMessage { target_user_id, content },
        }
    }
}

#[derive(Debug, Clone)]
pub enum CallState {
    Idle,
    Outgoing(String),
    Incoming(String),
    Connected(String),
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl Default for CallState {
    fn default() -> Self {
        CallState::Idle
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
}