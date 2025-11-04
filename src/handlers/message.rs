use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use crate::models::client::ClientRegistry;
use crate::models::error::ConnectionError;
use crate::models::message::{ClientId, Message, MessageType};

pub struct MessageHandler {
    clients: Arc<RwLock<ClientRegistry>>,
}

impl MessageHandler {
    pub fn new(clients: Arc<RwLock<ClientRegistry>>) -> Self {
        Self { clients }
    }

    /// Handle TextChat messages with routing logic
    #[instrument(skip(self, content), fields(sender_id = %sender_id, target_user_id = ?target_user_id, content_length = content.len()))]
    pub async fn handle_text_chat(
        &self,
        sender_id: ClientId,
        target_user_id: Option<ClientId>,
        content: String,
    ) -> Result<(), ConnectionError> {
        let message_type = MessageType::TextChat {
            target_user_id: target_user_id.clone(),
            content,
        };
        let message = Message::new_simple(Some(sender_id.clone()), message_type);

        let result = match &target_user_id {
            Some(target_id) => {
                debug!(
                    sender_id = %sender_id,
                    target_user_id = %target_id,
                    "Routing direct text message"
                );
                // Direct message to specific user
                self.send_to_user(target_id, message).await
            }
            None => {
                let recipient_count = self.get_broadcast_recipient_count(&sender_id).await;
                debug!(
                    sender_id = %sender_id,
                    recipient_count = recipient_count,
                    "Broadcasting text message"
                );
                // Broadcast to all users except sender
                self.broadcast_message(&sender_id, message).await
            }
        };

        match &result {
            Ok(()) => {
                info!(
                    sender_id = %sender_id,
                    target_user_id = ?target_user_id,
                    "Text chat message delivered successfully"
                );
            }
            Err(e) => {
                error!(
                    sender_id = %sender_id,
                    target_user_id = ?target_user_id,
                    error = %e,
                    "Text chat message delivery failed"
                );
            }
        }

        result
    }

    /// Send message to a specific user
    #[instrument(skip(self, message), fields(target_id = %target_id, message_id = %message.id))]
    async fn send_to_user(
        &self,
        target_id: &ClientId,
        message: Message,
    ) -> Result<(), ConnectionError> {
        let clients = self.clients.read().await;

        match clients.get(target_id) {
            Some(client) => {
                client.sender.send(message).map_err(|_| {
                    error!(
                        target_id = %target_id,
                        "Channel send failed for direct message"
                    );
                    ConnectionError::DeliveryFailed(format!(
                        "Failed to deliver message to user {}",
                        target_id
                    ))
                })?;
                debug!(
                    target_id = %target_id,
                    "Direct message sent successfully"
                );
                Ok(())
            }
            None => {
                warn!(
                    target_id = %target_id,
                    "Target user not found for direct message"
                );
                Err(ConnectionError::ClientNotFound(target_id.clone()))
            }
        }
    }

    /// Broadcast message to all connected clients except the sender
    #[instrument(skip(self, message), fields(sender_id = %sender_id, message_id = %message.id))]
    async fn broadcast_message(
        &self,
        sender_id: &ClientId,
        message: Message,
    ) -> Result<(), ConnectionError> {
        let clients = self.clients.read().await;
        let mut failed_deliveries = Vec::new();
        let mut successful_deliveries = 0;

        for (client_id, client) in clients.iter() {
            // Don't send message back to sender
            if client_id == sender_id {
                continue;
            }

            match client.sender.send(message.clone()) {
                Ok(()) => {
                    successful_deliveries += 1;
                    debug!(
                        recipient_id = %client_id,
                        "Broadcast message delivered"
                    );
                }
                Err(_) => {
                    failed_deliveries.push(client_id.clone());
                    warn!(
                        recipient_id = %client_id,
                        "Broadcast message delivery failed"
                    );
                }
            }
        }

        info!(
            sender_id = %sender_id,
            successful_deliveries = successful_deliveries,
            failed_deliveries = failed_deliveries.len(),
            "Broadcast message delivery completed"
        );

        if !failed_deliveries.is_empty() {
            return Err(ConnectionError::DeliveryFailed(format!(
                "Failed to deliver broadcast message to {} clients: {:?}",
                failed_deliveries.len(),
                failed_deliveries
            )));
        }

        Ok(())
    }

    /// Get the number of clients that would receive a broadcast message (excluding sender)
    pub async fn get_broadcast_recipient_count(&self, sender_id: &ClientId) -> usize {
        let clients = self.clients.read().await;
        clients
            .iter()
            .filter(|(client_id, _)| *client_id != sender_id)
            .count()
    }

    /// Check if a target user exists and is connected
    pub async fn user_exists(&self, user_id: &ClientId) -> bool {
        let clients = self.clients.read().await;
        clients.contains_key(user_id)
    }

    /// Handle generic messages for arbitrary text messages and commands
    #[instrument(skip(self, content), fields(sender_id = %sender_id, target_user_id = %target_user_id, content_length = content.len()))]
    pub async fn handle_generic_message(
        &self,
        sender_id: ClientId,
        target_user_id: ClientId,
        content: String,
    ) -> Result<(), ConnectionError> {
        let message_type = MessageType::GenericMessage {
            target_user_id: target_user_id.clone(),
            content,
        };

        // Create message with sender identification
        let message = Message::new_simple(Some(sender_id.clone()), message_type);

        debug!(
            sender_id = %sender_id,
            target_user_id = %target_user_id,
            message_id = %message.id,
            "Routing generic message"
        );

        // Send to the target user
        let result = self.send_to_user(&target_user_id, message).await;

        match &result {
            Ok(()) => {
                info!(
                    sender_id = %sender_id,
                    target_user_id = %target_user_id,
                    "Generic message delivered successfully"
                );
            }
            Err(e) => {
                error!(
                    sender_id = %sender_id,
                    target_user_id = %target_user_id,
                    error = %e,
                    "Generic message delivery failed"
                );
            }
        }

        result
    }

    /// Validate generic message content (basic validation)
    pub fn validate_generic_content(content: &str) -> bool {
        // Basic validation - ensure content is not empty and within reasonable limits
        !content.is_empty() && content.len() <= 10000 // 10KB limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::client::Client;
    use std::collections::HashMap;
    use tokio::sync::mpsc;

    async fn setup_test_clients() -> (
        Arc<RwLock<ClientRegistry>>,
        Vec<(ClientId, mpsc::UnboundedReceiver<Message>)>,
    ) {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let mut receivers = Vec::new();

        // Create 3 test clients
        for i in 1..=3 {
            let client_id = format!("client_{}", i);
            let (sender, receiver) = mpsc::unbounded_channel();
            let client = Client::new(client_id.clone(), sender);

            clients.write().await.insert(client_id.clone(), client);
            receivers.push((client_id, receiver));
        }

        (clients, receivers)
    }

    #[tokio::test]
    async fn test_direct_message_success() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let result = handler
            .handle_text_chat(
                "client_1".to_string(),
                Some("client_2".to_string()),
                "Hello client 2!".to_string(),
            )
            .await;

        assert!(result.is_ok());

        // Check that client_2 received the message
        let (_, ref mut client_2_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        assert_eq!(received_message.sender_id, Some("client_1".to_string()));

        match received_message.message_type {
            MessageType::TextChat {
                target_user_id,
                content,
            } => {
                assert_eq!(target_user_id, Some("client_2".to_string()));
                assert_eq!(content, "Hello client 2!");
            }
            _ => panic!("Expected TextChat message type"),
        }
    }

    #[tokio::test]
    async fn test_direct_message_user_not_found() {
        let (clients, _receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let result = handler
            .handle_text_chat(
                "client_1".to_string(),
                Some("non_existent_user".to_string()),
                "Hello nobody!".to_string(),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectionError::ClientNotFound(client_id) => {
                assert_eq!(client_id, "non_existent_user");
            }
            _ => panic!("Expected ClientNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_message() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let result = handler
            .handle_text_chat("client_1".to_string(), None, "Hello everyone!".to_string())
            .await;

        assert!(result.is_ok());

        // Check that client_2 and client_3 received the message, but not client_1
        for (client_id, receiver) in receivers.iter_mut() {
            if client_id == "client_1" {
                // Sender should not receive their own broadcast
                let result =
                    tokio::time::timeout(std::time::Duration::from_millis(50), receiver.recv())
                        .await;
                assert!(result.is_err()); // Should timeout
            } else {
                // Other clients should receive the message
                let received_message = receiver.recv().await.unwrap();
                assert_eq!(received_message.sender_id, Some("client_1".to_string()));

                match received_message.message_type {
                    MessageType::TextChat {
                        target_user_id,
                        content,
                    } => {
                        assert_eq!(target_user_id, None);
                        assert_eq!(content, "Hello everyone!");
                    }
                    _ => panic!("Expected TextChat message type"),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_broadcast_with_single_client() {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let handler = MessageHandler::new(clients.clone());

        // Add only one client
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let client = Client::new("solo_client".to_string(), sender);
        clients
            .write()
            .await
            .insert("solo_client".to_string(), client);

        let result = handler
            .handle_text_chat(
                "solo_client".to_string(),
                None,
                "Talking to myself".to_string(),
            )
            .await;

        assert!(result.is_ok());

        // Solo client should not receive their own broadcast
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(50), receiver.recv()).await;
        assert!(result.is_err()); // Should timeout
    }

    #[tokio::test]
    async fn test_get_broadcast_recipient_count() {
        let (clients, _receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let count = handler
            .get_broadcast_recipient_count(&"client_1".to_string())
            .await;
        assert_eq!(count, 2); // client_2 and client_3

        let count = handler
            .get_broadcast_recipient_count(&"non_existent".to_string())
            .await;
        assert_eq!(count, 3); // All clients since sender doesn't exist
    }

    #[tokio::test]
    async fn test_user_exists() {
        let (clients, _receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        assert!(handler.user_exists(&"client_1".to_string()).await);
        assert!(handler.user_exists(&"client_2".to_string()).await);
        assert!(handler.user_exists(&"client_3".to_string()).await);
        assert!(!handler.user_exists(&"non_existent".to_string()).await);
    }

    #[tokio::test]
    async fn test_send_to_user_direct() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let message = Message::new_simple(
            Some("sender".to_string()),
            MessageType::TextChat {
                target_user_id: Some("client_1".to_string()),
                content: "Direct test".to_string(),
            },
        );

        let result = handler
            .send_to_user(&"client_1".to_string(), message.clone())
            .await;
        assert!(result.is_ok());

        let (_, ref mut client_1_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_1")
            .unwrap();

        let received_message = client_1_receiver.recv().await.unwrap();
        assert_eq!(received_message.id, message.id);
    }

    #[tokio::test]
    async fn test_broadcast_message_direct() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let message = Message::new_simple(
            Some("client_1".to_string()),
            MessageType::TextChat {
                target_user_id: None,
                content: "Broadcast test".to_string(),
            },
        );

        let result = handler
            .broadcast_message(&"client_1".to_string(), message.clone())
            .await;
        assert!(result.is_ok());

        // Check that client_2 and client_3 received the message
        for (client_id, receiver) in receivers.iter_mut() {
            if client_id != "client_1" {
                let received_message = receiver.recv().await.unwrap();
                assert_eq!(received_message.id, message.id);
            }
        }
    }

    #[tokio::test]
    async fn test_message_with_sender_identification() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let sender_id = "client_1".to_string();
        let result = handler
            .handle_text_chat(
                sender_id.clone(),
                Some("client_2".to_string()),
                "Test message".to_string(),
            )
            .await;

        assert!(result.is_ok());

        let (_, ref mut client_2_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        assert_eq!(received_message.sender_id, Some(sender_id));
        assert!(!received_message.id.is_empty());
        assert!(received_message.timestamp <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn test_concurrent_message_handling() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = Arc::new(MessageHandler::new(clients));
        let mut handles = Vec::new();

        // Send multiple messages concurrently
        for i in 0..10 {
            let handler_clone = Arc::clone(&handler);
            let handle = tokio::spawn(async move {
                handler_clone
                    .handle_text_chat(
                        "client_1".to_string(),
                        Some("client_2".to_string()),
                        format!("Message {}", i),
                    )
                    .await
            });
            handles.push(handle);
        }

        // Wait for all messages to be sent
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Verify client_2 received all messages
        let (_, ref mut client_2_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let mut received_count = 0;
        while let Ok(message) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            client_2_receiver.recv(),
        )
        .await
        {
            if message.is_some() {
                received_count += 1;
            } else {
                break;
            }
        }

        assert_eq!(received_count, 10);
    }

    #[tokio::test]
    async fn test_generic_message_success() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let result = handler
            .handle_generic_message(
                "client_1".to_string(),
                "client_2".to_string(),
                "custom command data".to_string(),
            )
            .await;

        assert!(result.is_ok());

        // Check that client_2 received the generic message
        let (_, ref mut client_2_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        assert_eq!(received_message.sender_id, Some("client_1".to_string()));

        match received_message.message_type {
            MessageType::GenericMessage {
                target_user_id,
                content,
            } => {
                assert_eq!(target_user_id, "client_2");
                assert_eq!(content, "custom command data");
            }
            _ => panic!("Expected GenericMessage message type"),
        }
    }

    #[tokio::test]
    async fn test_generic_message_user_not_found() {
        let (clients, _receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let result = handler
            .handle_generic_message(
                "client_1".to_string(),
                "non_existent_user".to_string(),
                "command for nobody".to_string(),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectionError::ClientNotFound(client_id) => {
                assert_eq!(client_id, "non_existent_user");
            }
            _ => panic!("Expected ClientNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_generic_message_with_sender_identification() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let sender_id = "client_1".to_string();
        let result = handler
            .handle_generic_message(
                sender_id.clone(),
                "client_2".to_string(),
                "custom protocol message".to_string(),
            )
            .await;

        assert!(result.is_ok());

        let (_, ref mut client_2_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        assert_eq!(received_message.sender_id, Some(sender_id));
        assert!(!received_message.id.is_empty());
        assert!(received_message.timestamp <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn test_generic_message_preserve_content() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let complex_content =
            r#"{"command": "custom_action", "params": {"key": "value", "array": [1, 2, 3]}}"#;

        let result = handler
            .handle_generic_message(
                "client_1".to_string(),
                "client_2".to_string(),
                complex_content.to_string(),
            )
            .await;

        assert!(result.is_ok());

        let (_, ref mut client_2_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let received_message = client_2_receiver.recv().await.unwrap();
        match received_message.message_type {
            MessageType::GenericMessage { content, .. } => {
                assert_eq!(content, complex_content);
            }
            _ => panic!("Expected GenericMessage message type"),
        }
    }

    #[tokio::test]
    async fn test_validate_generic_content() {
        // Valid content
        assert!(MessageHandler::validate_generic_content("valid content"));
        assert!(MessageHandler::validate_generic_content("command:action"));
        assert!(MessageHandler::validate_generic_content(&"a".repeat(1000))); // 1KB content

        // Invalid content
        assert!(!MessageHandler::validate_generic_content("")); // Empty content
        assert!(!MessageHandler::validate_generic_content(
            &"a".repeat(10001)
        )); // Too large (>10KB)
    }

    #[tokio::test]
    async fn test_generic_message_different_content_types() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = MessageHandler::new(clients);

        let test_contents = vec![
            "simple text",
            "command:start_session",
            r#"{"json": "data"}"#,
            "XML:<root><item>value</item></root>",
            "base64:SGVsbG8gV29ybGQ=",
            "multi\nline\ncontent",
        ];

        for (i, content) in test_contents.iter().enumerate() {
            let result = handler
                .handle_generic_message(
                    "client_1".to_string(),
                    "client_2".to_string(),
                    content.to_string(),
                )
                .await;

            assert!(result.is_ok(), "Failed for content type {}: {}", i, content);

            let (_, ref mut client_2_receiver) = receivers
                .iter_mut()
                .find(|(id, _)| id == "client_2")
                .unwrap();

            let received_message = client_2_receiver.recv().await.unwrap();
            match received_message.message_type {
                MessageType::GenericMessage {
                    content: received_content,
                    ..
                } => {
                    assert_eq!(received_content, *content);
                }
                _ => panic!("Expected GenericMessage message type"),
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_generic_messages() {
        let (clients, mut receivers) = setup_test_clients().await;
        let handler = Arc::new(MessageHandler::new(clients));
        let mut handles = Vec::new();

        // Send multiple generic messages concurrently
        for i in 0..10 {
            let handler_clone = Arc::clone(&handler);
            let handle = tokio::spawn(async move {
                handler_clone
                    .handle_generic_message(
                        "client_1".to_string(),
                        "client_2".to_string(),
                        format!("command_{}", i),
                    )
                    .await
            });
            handles.push(handle);
        }

        // Wait for all messages to be sent
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Verify client_2 received all messages
        let (_, ref mut client_2_receiver) = receivers
            .iter_mut()
            .find(|(id, _)| id == "client_2")
            .unwrap();

        let mut received_count = 0;
        let mut received_commands = Vec::new();

        while let Ok(message) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            client_2_receiver.recv(),
        )
        .await
        {
            if let Some(msg) = message {
                received_count += 1;
                if let MessageType::GenericMessage { content, .. } = msg.message_type {
                    received_commands.push(content);
                }
            } else {
                break;
            }
        }

        assert_eq!(received_count, 10);

        // Verify all commands were received
        for i in 0..10 {
            let expected_command = format!("command_{}", i);
            assert!(received_commands.contains(&expected_command));
        }
    }
}
