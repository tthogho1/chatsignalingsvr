use chrono::{DateTime, Utc};
use tokio::sync::mpsc::UnboundedSender;
use std::collections::HashMap;

use super::message::{ClientId, Message};

#[derive(Debug, Clone)]
pub struct Client {
    pub id: ClientId,
    pub username: Option<String>,
    pub sender: UnboundedSender<Message>,
    pub connected_at: DateTime<Utc>,
}

impl Client {
    pub fn new(id: ClientId, sender: UnboundedSender<Message>) -> Self {
        Self {
            id,
            username: None,
            sender,
            connected_at: Utc::now(),
        }
    }

    pub fn new_with_username(id: ClientId, username: String, sender: UnboundedSender<Message>) -> Self {
        Self {
            id,
            username: Some(username),
            sender,
            connected_at: Utc::now(),
        }
    }

    pub fn set_username(&mut self, username: String) {
        self.username = Some(username);
    }

    pub fn get_username(&self) -> Option<&String> {
        self.username.as_ref()
    }
}

pub type ClientRegistry = HashMap<ClientId, Client>;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use crate::models::message::MessageType;

    #[test]
    fn test_client_creation() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let client_id = "test_client_123".to_string();
        let client = Client::new(client_id.clone(), sender);

        assert_eq!(client.id, client_id);
        assert!(client.connected_at <= Utc::now());
    }

    #[test]
    fn test_client_registry_operations() {
        let mut registry: ClientRegistry = HashMap::new();
        let (sender1, _receiver1) = mpsc::unbounded_channel();
        let (sender2, _receiver2) = mpsc::unbounded_channel();

        let client1 = Client::new("client1".to_string(), sender1);
        let client2 = Client::new("client2".to_string(), sender2);

        // Test insertion
        registry.insert(client1.id.clone(), client1.clone());
        registry.insert(client2.id.clone(), client2.clone());

        assert_eq!(registry.len(), 2);
        assert!(registry.contains_key("client1"));
        assert!(registry.contains_key("client2"));

        // Test retrieval
        let retrieved_client1 = registry.get("client1").unwrap();
        assert_eq!(retrieved_client1.id, "client1");

        // Test removal
        let removed_client = registry.remove("client1");
        assert!(removed_client.is_some());
        assert_eq!(registry.len(), 1);
        assert!(!registry.contains_key("client1"));
    }

    #[tokio::test]
    async fn test_client_message_sending() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let client = Client::new("test_client".to_string(), sender);

        let message_type = MessageType::TextChat {
            target_user_id: Some("target".to_string()),
            content: "Test message".to_string(),
        };
        let message = Message::new(Some("sender".to_string()), message_type);

        // Test sending message through client's channel
        let send_result = client.sender.send(message.clone());
        assert!(send_result.is_ok());

        // Test receiving the message
        let received_message = receiver.recv().await;
        assert!(received_message.is_some());
        let received = received_message.unwrap();
        assert_eq!(received.id, message.id);
        assert_eq!(received.sender_id, message.sender_id);
    }

    #[test]
    fn test_client_clone() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let client = Client::new("cloneable_client".to_string(), sender);
        let cloned_client = client.clone();

        assert_eq!(client.id, cloned_client.id);
        assert_eq!(client.connected_at, cloned_client.connected_at);
        // Note: UnboundedSender implements Clone, so this should work
    }

    #[test]
    fn test_client_debug_formatting() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let client = Client::new("debug_client".to_string(), sender);
        let debug_string = format!("{:?}", client);

        assert!(debug_string.contains("Client"));
        assert!(debug_string.contains("debug_client"));
        assert!(debug_string.contains("connected_at"));
    }

    #[test]
    fn test_client_registry_concurrent_access() {
        use std::sync::{Arc, RwLock};
        use std::thread;

        let registry = Arc::new(RwLock::new(ClientRegistry::new()));
        let mut handles = vec![];

        // Spawn multiple threads to test concurrent access
        for i in 0..5 {
            let registry_clone = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                let (sender, _receiver) = mpsc::unbounded_channel();
                let client = Client::new(format!("client_{}", i), sender);
                
                // Write to registry
                {
                    let mut reg = registry_clone.write().unwrap();
                    reg.insert(client.id.clone(), client);
                }

                // Read from registry
                {
                    let reg = registry_clone.read().unwrap();
                    assert!(reg.contains_key(&format!("client_{}", i)));
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state
        let final_registry = registry.read().unwrap();
        assert_eq!(final_registry.len(), 5);
    }

    #[tokio::test]
    async fn test_client_channel_closure() {
        let (sender, receiver) = mpsc::unbounded_channel();
        let client = Client::new("test_client".to_string(), sender);

        // Drop the receiver to close the channel
        drop(receiver);

        let message_type = MessageType::TextChat {
            target_user_id: None,
            content: "This should fail".to_string(),
        };
        let message = Message::new(Some("sender".to_string()), message_type);

        // Sending should fail because receiver is dropped
        let send_result = client.sender.send(message);
        assert!(send_result.is_err());
    }
}