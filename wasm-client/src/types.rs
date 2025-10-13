use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: String,
    pub sender: String,
    pub target: Option<String>,
    #[serde(rename = "signaling_type")]
    pub signaling_type: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignalingData {
    pub sdp: Option<String>,
    pub candidate: Option<String>,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
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