use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type MessageId = String;
pub type ClientId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    TextChat {
        target_user_id: Option<ClientId>,
        content: String,
    },
    WebRTCSignaling {
        target_user_id: ClientId,
        signaling_data: serde_json::Value,
    },
    GenericMessage {
        target_user_id: ClientId,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub sender_id: Option<ClientId>,
    pub sender_username: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
}

impl Message {
    pub fn new(sender_id: Option<ClientId>, sender_username: Option<String>, message_type: MessageType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id,
            sender_username,
            timestamp: Utc::now(),
            message_type,
        }
    }
    
    // Backward compatibility method for existing code
    pub fn new_simple(sender_id: Option<ClientId>, message_type: MessageType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id,
            sender_username: None,
            timestamp: Utc::now(),
            message_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_creation() {
        let sender_id = Some("client123".to_string());
        let message_type = MessageType::TextChat {
            target_user_id: Some("client456".to_string()),
            content: "Hello, World!".to_string(),
        };

        let message = Message::new_simple(sender_id.clone(), message_type.clone());

        assert_eq!(message.sender_id, sender_id);
        assert!(matches!(message.message_type, MessageType::TextChat { .. }));
        assert!(!message.id.is_empty());
        assert!(message.timestamp <= Utc::now());
    }

    #[test]
    fn test_text_chat_serialization() {
        let message_type = MessageType::TextChat {
            target_user_id: Some("user123".to_string()),
            content: "Test message".to_string(),
        };
        let message = Message::new_simple(Some("sender456".to_string()), message_type);

        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");
        let deserialized: Message = serde_json::from_str(&serialized).expect("Failed to deserialize message");

        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.sender_id, deserialized.sender_id);
        assert_eq!(message.timestamp, deserialized.timestamp);
        
        match (&message.message_type, &deserialized.message_type) {
            (MessageType::TextChat { target_user_id: t1, content: c1 }, 
             MessageType::TextChat { target_user_id: t2, content: c2 }) => {
                assert_eq!(t1, t2);
                assert_eq!(c1, c2);
            }
            _ => panic!("Message type mismatch after serialization/deserialization"),
        }
    }

    #[test]
    fn test_text_chat_broadcast_serialization() {
        let message_type = MessageType::TextChat {
            target_user_id: None,
            content: "Broadcast message".to_string(),
        };
        let message = Message::new_simple(Some("broadcaster".to_string()), message_type);

        let serialized = serde_json::to_string(&message).expect("Failed to serialize broadcast message");
        let deserialized: Message = serde_json::from_str(&serialized).expect("Failed to deserialize broadcast message");

        match deserialized.message_type {
            MessageType::TextChat { target_user_id: None, content } => {
                assert_eq!(content, "Broadcast message");
            }
            _ => panic!("Expected TextChat with None target_user_id"),
        }
    }

    #[test]
    fn test_webrtc_signaling_serialization() {
        let signaling_data = json!({
            "type": "offer",
            "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\n"
        });

        let message_type = MessageType::WebRTCSignaling {
            target_user_id: "peer123".to_string(),
            signaling_data: signaling_data.clone(),
        };
        let message = Message::new_simple(Some("caller456".to_string()), message_type);

        let serialized = serde_json::to_string(&message).expect("Failed to serialize WebRTC message");
        let deserialized: Message = serde_json::from_str(&serialized).expect("Failed to deserialize WebRTC message");

        match deserialized.message_type {
            MessageType::WebRTCSignaling { target_user_id, signaling_data: data } => {
                assert_eq!(target_user_id, "peer123");
                assert_eq!(data, signaling_data);
            }
            _ => panic!("Expected WebRTCSignaling message type"),
        }
    }

    #[test]
    fn test_generic_message_serialization() {
        let message_type = MessageType::GenericMessage {
            target_user_id: "target789".to_string(),
            content: "Custom command data".to_string(),
        };
        let message = Message::new_simple(Some("sender123".to_string()), message_type);

        let serialized = serde_json::to_string(&message).expect("Failed to serialize generic message");
        let deserialized: Message = serde_json::from_str(&serialized).expect("Failed to deserialize generic message");

        match deserialized.message_type {
            MessageType::GenericMessage { target_user_id, content } => {
                assert_eq!(target_user_id, "target789");
                assert_eq!(content, "Custom command data");
            }
            _ => panic!("Expected GenericMessage message type"),
        }
    }

    #[test]
    fn test_message_with_no_sender() {
        let message_type = MessageType::TextChat {
            target_user_id: Some("user123".to_string()),
            content: "System message".to_string(),
        };
        let message = Message::new_simple(None, message_type);

        assert!(message.sender_id.is_none());

        let serialized = serde_json::to_string(&message).expect("Failed to serialize message with no sender");
        let deserialized: Message = serde_json::from_str(&serialized).expect("Failed to deserialize message with no sender");

        assert!(deserialized.sender_id.is_none());
    }

    #[test]
    fn test_invalid_json_deserialization() {
        let invalid_json = r#"{"id": "123", "invalid_field": true}"#;
        let result: Result<Message, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_type_variants() {
        // Test that all MessageType variants can be created and serialized
        let variants = vec![
            MessageType::TextChat {
                target_user_id: Some("user1".to_string()),
                content: "Direct message".to_string(),
            },
            MessageType::TextChat {
                target_user_id: None,
                content: "Broadcast message".to_string(),
            },
            MessageType::WebRTCSignaling {
                target_user_id: "peer1".to_string(),
                signaling_data: json!({"type": "ice-candidate"}),
            },
            MessageType::GenericMessage {
                target_user_id: "target1".to_string(),
                content: "Generic content".to_string(),
            },
        ];

        for message_type in variants {
            let message = Message::new_simple(Some("sender".to_string()), message_type);
            let serialized = serde_json::to_string(&message).expect("Failed to serialize message variant");
            let _deserialized: Message = serde_json::from_str(&serialized).expect("Failed to deserialize message variant");
        }
    }
}