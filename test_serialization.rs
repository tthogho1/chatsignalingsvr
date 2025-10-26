use serde::{Deserialize, Serialize};
use serde_json;

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
    pub timestamp: String,
    pub message_type: MessageType,
}

fn main() {
    let signaling_data = serde_json::json!({
        "type": "call-request"
    });

    let message_type = MessageType::WebRTCSignaling {
        target_user_id: "TEST2".to_string(),
        signaling_data: signaling_data.clone(),
    };

    let message = Message {
        id: "496f9d0b-5f8c-4b70-b8eb-a108eaba0ae0".to_string(),
        sender_id: Some("TEST".to_string()),
        timestamp: "2025-10-24T08:14:28.133Z".to_string(),
        message_type,
    };

    let serialized = serde_json::to_string_pretty(&message).expect("Failed to serialize");
    println!("Correct JSON format:");
    println!("{}", serialized);

    // Test deserialization
    let deserialized: Message = serde_json::from_str(&serialized).expect("Failed to deserialize");
    println!("\nDeserialization successful: {:?}", deserialized);
}