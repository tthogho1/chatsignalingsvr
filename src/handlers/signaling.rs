use std::sync::Arc;
use tracing::{info, debug, warn, error, instrument};

use crate::models::message::{ClientId, Message, MessageType};
use crate::models::error::ConnectionError;
use crate::handlers::connection::ConnectionManager;

pub struct SignalingHandler {
    connection_manager: Arc<ConnectionManager>,
}

impl SignalingHandler {
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    /// Handle WebRTC signaling messages (ICE candidates, SDP offers/answers)
    /// Now accepts username string for target and resolves to ClientId internally
    #[instrument(skip(self, signaling_data), fields(sender_id = %sender_id, target_username = %target_username, signaling_type = ?signaling_data.get("type")))]
    pub async fn handle_webrtc_signaling(
        &self,
        sender_id: ClientId,
        target_username: String,
        signaling_data: serde_json::Value,
    ) -> Result<(), ConnectionError> {
        // Extract type for logging before moving signaling_data
        let signaling_type = signaling_data.get("type").cloned();
        
        // Look up the target client ID by username using ConnectionManager
        let target_client_id = self.connection_manager.find_client_by_username(&target_username).await
            .ok_or_else(|| {
                warn!(
                    sender_id = %sender_id,
                    target_username = %target_username,
                    "Target user not found by username for signaling message"
                );
                ConnectionError::ClientNotFound(target_username.clone())
            })?;

        info!(
            sender_id = %sender_id,
            target_username = %target_username,
            target_client_id = %target_client_id,
            signaling_type = ?signaling_type,
            signaling_data = ?signaling_data,
            "=== WebRTC Username Resolution Success ==="
        );
        
        // Preserve the original signaling data structure
        let message_type = MessageType::WebRTCSignaling {
            target_user_id: target_client_id.clone(),
            signaling_data,
        };
        
        // Get sender's username
        let sender_username = self.connection_manager.find_username_by_client_id(&sender_id).await;
        
        // Create message with sender identification
        let message = Message::new(Some(sender_id.clone()), sender_username, message_type);
        let message_id = message.id.clone(); // IDを事前にコピー

        debug!(
            sender_id = %sender_id,
            target_username = %target_username,
            target_client_id = %target_client_id,
            message_id = %message_id,
            signaling_type = ?signaling_type,
            "Forwarding WebRTC signaling message"
        );

        // Forward to the target user
        let result = self.forward_signaling_message(&target_client_id, message).await;

        match &result {
            Ok(()) => {
                info!(
                    sender_id = %sender_id,
                    target_username = %target_username,
                    target_client_id = %target_client_id,
                    signaling_type = ?signaling_type,
                    message_id = %message_id,
                    "=== WebRTC Message Delivery Success ==="
                );
            }
            Err(e) => {
                error!(
                    sender_id = %sender_id,
                    target_username = %target_username,
                    target_client_id = %target_client_id,
                    signaling_type = ?signaling_type,
                    error = %e,
                    error_details = ?e,
                    "=== WebRTC Message Delivery Failed ==="
                );
            }
        }

        result
    }

    /// Forward signaling message to the target user
    #[instrument(skip(self, message), fields(target_id = %target_id, message_id = %message.id))]
    async fn forward_signaling_message(
        &self,
        target_id: &ClientId,
        message: Message,
    ) -> Result<(), ConnectionError> {
        // Use ConnectionManager to send the message
        self.connection_manager.send_to_client(target_id, message).await
    }

    /// Check if a target user exists for signaling
    /// Validate signaling data structure (basic validation)
    pub fn validate_signaling_data(signaling_data: &serde_json::Value) -> bool {
        // Basic validation - ensure it's an object and not null
        signaling_data.is_object() && !signaling_data.is_null()
    }

    /// Handle different types of WebRTC signaling messages
    pub async fn handle_ice_candidate(
        &self,
        sender_id: ClientId,
        target_username: String,
        candidate_data: serde_json::Value,
    ) -> Result<(), ConnectionError> {
        // Validate that this looks like an ICE candidate
        if !candidate_data.is_object() {
            return Err(ConnectionError::InvalidMessage(
                "ICE candidate data must be an object".to_string()
            ));
        }

        self.handle_webrtc_signaling(sender_id, target_username, candidate_data).await
    }

    /// Handle SDP offer/answer messages
    pub async fn handle_sdp_message(
        &self,
        sender_id: ClientId,
        target_username: String,
        sdp_data: serde_json::Value,
    ) -> Result<(), ConnectionError> {
        // Validate that this looks like SDP data
        if !sdp_data.is_object() {
            return Err(ConnectionError::InvalidMessage(
                "SDP data must be an object".to_string()
            ));
        }

        // Check for required SDP fields (type and sdp)
        if !sdp_data.get("type").is_some() || !sdp_data.get("sdp").is_some() {
            return Err(ConnectionError::InvalidMessage(
                "SDP data must contain 'type' and 'sdp' fields".to_string()
            ));
        }

        self.handle_webrtc_signaling(sender_id, target_username, sdp_data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use crate::models::client::Client;
    use std::collections::HashMap;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use crate::models::client::ClientRegistry;

    async fn setup_test_clients() -> (Arc<ConnectionManager>, Vec<(ClientId, mpsc::UnboundedReceiver<Message>)>) {
        let connection_manager = Arc::new(ConnectionManager::new());
        let mut receivers = Vec::new();

        // Create 2 test clients for peer-to-peer signaling
        for i in 1..=2 {
            let client_id = format!("client_{}", i);
            let username = client_id.clone(); // Use client_id as username for testing
            let (sender, receiver) = mpsc::unbounded_channel();
            let mut client = Client::new(client_id.clone(), sender);
            client.username = Some(username); // Set username for test
            
            // Add clients to the connection manager
            connection_manager.add_client(client).await;
            receivers.push((client_id, receiver));
        }

        (connection_manager, receivers)
    }

    #[tokio::test]
    async fn test_webrtc_signaling_success() {
        let (connection_manager, mut receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let signaling_data = json!({
            "type": "offer",
            "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\n"
        });

        let result = handler.handle_webrtc_signaling(
            "client_1".to_string(),
            "client_2".to_string(),  // Use client_id same as username
            signaling_data.clone(),
        ).await;

        assert!(result.is_ok());

        // Check that client_2 received the signaling message
        let (_, ref mut client_2_receiver) = receivers.iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        assert_eq!(received_message.sender_id, Some("client_1".to_string()));
        
        match received_message.message_type {
            MessageType::WebRTCSignaling { target_user_id, signaling_data: data } => {
                assert_eq!(target_user_id, "client_2");
                assert_eq!(data, signaling_data);
            }
            _ => panic!("Expected WebRTCSignaling message type"),
        }
    }

    #[tokio::test]
    async fn test_webrtc_signaling_user_not_found() {
        let (connection_manager, _receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let signaling_data = json!({
            "type": "offer",
            "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\n"
        });

        let result = handler.handle_webrtc_signaling(
            "client_1".to_string(),
            "non_existent_user".to_string(),
            signaling_data,
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectionError::ClientNotFound(client_id) => {
                assert_eq!(client_id, "non_existent_user");
            }
            _ => panic!("Expected ClientNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_ice_candidate_handling() {
        let (connection_manager, mut receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let ice_candidate = json!({
            "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54400 typ host",
            "sdpMid": "0",
            "sdpMLineIndex": 0
        });

        let result = handler.handle_ice_candidate(
            "client_1".to_string(),
            "client_2".to_string(),
            ice_candidate.clone(),
        ).await;

        assert!(result.is_ok());

        let (_, ref mut client_2_receiver) = receivers.iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        match received_message.message_type {
            MessageType::WebRTCSignaling { signaling_data, .. } => {
                assert_eq!(signaling_data, ice_candidate);
            }
            _ => panic!("Expected WebRTCSignaling message type"),
        }
    }

    #[tokio::test]
    async fn test_ice_candidate_invalid_data() {
        let (connection_manager, _receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let invalid_data = json!("not an object");

        let result = handler.handle_ice_candidate(
            "client_1".to_string(),
            "client_2".to_string(),
            invalid_data,
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectionError::InvalidMessage(msg) => {
                assert!(msg.contains("ICE candidate data must be an object"));
            }
            _ => panic!("Expected InvalidMessage error"),
        }
    }

    #[tokio::test]
    async fn test_sdp_offer_handling() {
        let (connection_manager, mut receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let sdp_offer = json!({
            "type": "offer",
            "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n"
        });

        let result = handler.handle_sdp_message(
            "client_1".to_string(),
            "client_2".to_string(),
            sdp_offer.clone(),
        ).await;

        assert!(result.is_ok());

        let (_, ref mut client_2_receiver) = receivers.iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        match received_message.message_type {
            MessageType::WebRTCSignaling { signaling_data, .. } => {
                assert_eq!(signaling_data, sdp_offer);
            }
            _ => panic!("Expected WebRTCSignaling message type"),
        }
    }

    #[tokio::test]
    async fn test_sdp_answer_handling() {
        let (connection_manager, mut receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let sdp_answer = json!({
            "type": "answer",
            "sdp": "v=0\r\no=- 987654321 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n"
        });

        let result = handler.handle_sdp_message(
            "client_2".to_string(),
            "client_1".to_string(),
            sdp_answer.clone(),
        ).await;

        assert!(result.is_ok());

        let (_, ref mut client_1_receiver) = receivers.iter_mut()
            .find(|(id, _)| id == "client_1")
            .unwrap();

        let received_message = client_1_receiver.recv().await.unwrap();
        match received_message.message_type {
            MessageType::WebRTCSignaling { signaling_data, .. } => {
                assert_eq!(signaling_data, sdp_answer);
            }
            _ => panic!("Expected WebRTCSignaling message type"),
        }
    }

    #[tokio::test]
    async fn test_sdp_invalid_data() {
        let (connection_manager, _receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let invalid_sdp = json!("not an object");

        let result = handler.handle_sdp_message(
            "client_1".to_string(),
            "client_2".to_string(),
            invalid_sdp,
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectionError::InvalidMessage(msg) => {
                assert!(msg.contains("SDP data must be an object"));
            }
            _ => panic!("Expected InvalidMessage error"),
        }
    }

    #[tokio::test]
    async fn test_sdp_missing_required_fields() {
        let (connection_manager, _receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let incomplete_sdp = json!({
            "type": "offer"
            // Missing "sdp" field
        });

        let result = handler.handle_sdp_message(
            "client_1".to_string(),
            "client_2".to_string(),
            incomplete_sdp,
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectionError::InvalidMessage(msg) => {
                assert!(msg.contains("SDP data must contain 'type' and 'sdp' fields"));
            }
            _ => panic!("Expected InvalidMessage error"),
        }
    }

    #[tokio::test]
    async fn test_validate_signaling_data() {
        // Valid signaling data
        let valid_data = json!({
            "type": "offer",
            "sdp": "v=0..."
        });
        assert!(SignalingHandler::validate_signaling_data(&valid_data));

        // Invalid signaling data
        let invalid_data = json!(null);
        assert!(!SignalingHandler::validate_signaling_data(&invalid_data));

        let invalid_data2 = json!("string");
        assert!(!SignalingHandler::validate_signaling_data(&invalid_data2));

        let invalid_data3 = json!(123);
        assert!(!SignalingHandler::validate_signaling_data(&invalid_data3));
    }

    #[tokio::test]
    async fn test_signaling_message_with_sender_identification() {
        let (connection_manager, mut receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let signaling_data = json!({
            "type": "ice-candidate",
            "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54400 typ host"
        });

        let sender_id = "client_1".to_string();
        let result = handler.handle_webrtc_signaling(
            sender_id.clone(),
            "client_2".to_string(),
            signaling_data,
        ).await;

        assert!(result.is_ok());

        let (_, ref mut client_2_receiver) = receivers.iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        assert_eq!(received_message.sender_id, Some(sender_id));
        assert!(!received_message.id.is_empty());
        assert!(received_message.timestamp <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn test_preserve_original_signaling_structure() {
        let (connection_manager, mut receivers) = setup_test_clients().await;
        let handler = SignalingHandler::new(connection_manager);

        let complex_signaling_data = json!({
            "type": "offer",
            "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\n",
            "custom_field": "custom_value",
            "nested": {
                "property": "value",
                "array": [1, 2, 3]
            }
        });

        let result = handler.handle_webrtc_signaling(
            "client_1".to_string(),
            "client_2".to_string(),
            complex_signaling_data.clone(),
        ).await;

        assert!(result.is_ok());

        let (_, ref mut client_2_receiver) = receivers.iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        match received_message.message_type {
            MessageType::WebRTCSignaling { signaling_data, .. } => {
                // Verify the entire structure is preserved
                assert_eq!(signaling_data, complex_signaling_data);
                assert_eq!(signaling_data["custom_field"], "custom_value");
                assert_eq!(signaling_data["nested"]["property"], "value");
                assert_eq!(signaling_data["nested"]["array"], json!([1, 2, 3]));
            }
            _ => panic!("Expected WebRTCSignaling message type"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_signaling_messages() {
        let (connection_manager, mut receivers) = setup_test_clients().await;
        let handler = Arc::new(SignalingHandler::new(connection_manager));
        let mut handles = Vec::new();

        // Send multiple signaling messages concurrently
        for i in 0..5 {
            let handler_clone = Arc::clone(&handler);
            let handle = tokio::spawn(async move {
                let signaling_data = json!({
                    "type": "ice-candidate",
                    "candidate": format!("candidate:{} 1 UDP 2130706431 192.168.1.100 5440{} typ host", i, i)
                });
                
                handler_clone.handle_webrtc_signaling(
                    "client_1".to_string(),
                    "client_2".to_string(),
                    signaling_data,
                ).await
            });
            handles.push(handle);
        }

        // Wait for all messages to be sent
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Verify client_2 received all messages
        let (_, ref mut client_2_receiver) = receivers.iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let mut received_count = 0;
        while let Ok(message) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            client_2_receiver.recv()
        ).await {
            if message.is_some() {
                received_count += 1;
            } else {
                break;
            }
        }

        assert_eq!(received_count, 5);
    }
}