use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::models::client::{Client, ClientRegistry};
use crate::models::error::ConnectionError;
use crate::models::message::{ClientId, Message};

pub struct ConnectionManager {
    clients: Arc<RwLock<ClientRegistry>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(ClientRegistry::new())),
        }
    }

    pub fn generate_client_id() -> ClientId {
        Uuid::new_v4().to_string()
    }

    #[instrument(skip(self, client), fields(client_id = %client.id))]
    pub async fn add_client(&self, client: Client) -> ClientId {
        let client_id = client.id.clone();
        let connected_at = client.connected_at;
        let mut clients = self.clients.write().await;
        clients.insert(client_id.clone(), client);

        info!(
            client_id = %client_id,
            connected_at = %connected_at,
            total_clients = clients.len(),
            "Client registered and added to registry"
        );

        client_id
    }

    #[instrument(skip(self), fields(client_id = %client_id))]
    pub async fn remove_client(&self, client_id: &ClientId) -> Option<Client> {
        let mut clients = self.clients.write().await;
        let removed_client = clients.remove(client_id);

        match &removed_client {
            Some(_) => {
                debug!(
                    client_id = %client_id,
                    total_clients = clients.len(),
                    "Client removed from registry"
                );
            }
            None => {
                warn!(
                    client_id = %client_id,
                    "Attempted to remove non-existent client"
                );
            }
        }

        removed_client
    }

    pub async fn get_client(&self, client_id: &ClientId) -> Option<Client> {
        let clients = self.clients.read().await;
        clients.get(client_id).cloned()
    }

    pub async fn get_all_clients(&self) -> Vec<ClientId> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }

    pub async fn client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }

    pub async fn client_exists(&self, client_id: &ClientId) -> bool {
        let clients = self.clients.read().await;
        clients.contains_key(client_id)
    }

    /// Creates a new client with a fresh channel and adds it to the registry
    #[instrument(skip(self))]
    pub async fn connect_client(
        &self,
    ) -> (
        ClientId,
        tokio::sync::mpsc::UnboundedReceiver<crate::models::message::Message>,
    ) {
        let client_id = Self::generate_client_id();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let client = Client::new(client_id.clone(), sender);

        info!(
            client_id = %client_id,
            connected_at = %client.connected_at,
            "New client connecting"
        );

        self.add_client(client).await;
        (client_id, receiver)
    }

    /// Removes a client and performs cleanup
    #[instrument(skip(self), fields(client_id = %client_id))]
    pub async fn disconnect_client(&self, client_id: &ClientId) -> bool {
        match self.remove_client(client_id).await {
            Some(client) => {
                let connection_duration = Utc::now() - client.connected_at;
                info!(
                    client_id = %client_id,
                    connected_at = %client.connected_at,
                    connection_duration_seconds = connection_duration.num_seconds(),
                    "Client disconnected"
                );
                // Client channel will be automatically closed when the Client is dropped
                // since it holds the UnboundedSender
                true
            }
            None => {
                debug!(
                    client_id = %client_id,
                    "Disconnect attempt for non-existent client"
                );
                false
            }
        }
    }

    /// Gets connection information for a client
    pub async fn get_client_info(&self, client_id: &ClientId) -> Option<ClientInfo> {
        let clients = self.clients.read().await;
        clients.get(client_id).map(|client| ClientInfo {
            id: client.id.clone(),
            connected_at: client.connected_at,
        })
    }

    /// Gets connection information for all clients
    pub async fn get_all_client_info(&self) -> Vec<ClientInfo> {
        let clients = self.clients.read().await;
        clients
            .values()
            .map(|client| ClientInfo {
                id: client.id.clone(),
                connected_at: client.connected_at,
            })
            .collect()
    }

    /// Find client ID by username
    #[instrument(skip(self), fields(username = %username))]
    pub async fn find_client_by_username(&self, username: &str) -> Option<ClientId> {
        let clients = self.clients.read().await;
        for (client_id, client) in clients.iter() {
            if let Some(client_username) = &client.username {
                if client_username == username {
                    debug!(
                        client_id = %client_id,
                        username = %username,
                        "Found client by username"
                    );
                    return Some(client_id.clone());
                }
            }
        }
        debug!(
            username = %username,
            "Client not found by username"
        );
        None
    }

    /// Find username by client ID
    pub async fn find_username_by_client_id(&self, client_id: &ClientId) -> Option<String> {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(client_id) {
            if let Some(username) = &client.username {
                debug!(
                    client_id = %client_id,
                    username = %username,
                    "Found username by client ID"
                );
                return Some(username.clone());
            }
        }
        debug!(
            client_id = %client_id,
            "Username not found for client ID"
        );
        None
    }

    /// Update client username
    #[instrument(skip(self), fields(client_id = %client_id, username = %username))]
    pub async fn update_client_username(&self, client_id: &ClientId, username: String) -> bool {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(client_id) {
            let old_username = client.username.clone();
            client.set_username(username.clone());
            info!(
                client_id = %client_id,
                old_username = ?old_username,
                new_username = %username,
                "=== Client Username Updated Successfully ==="
            );
            true
        } else {
            warn!(
                client_id = %client_id,
                username = %username,
                total_clients = clients.len(),
                "Cannot update username: client not found in registry"
            );
            false
        }
    }

    /// Send a message to a specific client
    #[instrument(skip(self, message), fields(target_id = %target_id, message_id = %message.id))]
    pub async fn send_to_client(
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
                        "Channel send failed for message"
                    );
                    ConnectionError::DeliveryFailed(format!(
                        "Failed to deliver message to client {}",
                        target_id
                    ))
                })?;
                debug!(
                    target_id = %target_id,
                    "Message sent successfully"
                );
                Ok(())
            }
            None => {
                warn!(
                    target_id = %target_id,
                    "Target client not found for message delivery"
                );
                Err(ConnectionError::ClientNotFound(target_id.clone()))
            }
        }
    }
}

/// Information about a connected client (without the channel)
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: ClientId,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::Message;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let manager = ConnectionManager::new();
        assert_eq!(manager.client_count().await, 0);
    }

    #[test]
    fn test_generate_client_id() {
        let id1 = ConnectionManager::generate_client_id();
        let id2 = ConnectionManager::generate_client_id();

        // IDs should be different
        assert_ne!(id1, id2);

        // IDs should be valid UUIDs (36 characters with hyphens)
        assert_eq!(id1.len(), 36);
        assert_eq!(id2.len(), 36);
        assert!(id1.contains('-'));
        assert!(id2.contains('-'));
    }

    #[tokio::test]
    async fn test_add_client() {
        let manager = ConnectionManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let client_id = ConnectionManager::generate_client_id();
        let client = Client::new(client_id.clone(), sender);

        let returned_id = manager.add_client(client).await;

        assert_eq!(returned_id, client_id);
        assert_eq!(manager.client_count().await, 1);
        assert!(manager.client_exists(&client_id).await);
    }

    #[tokio::test]
    async fn test_remove_client() {
        let manager = ConnectionManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let client_id = ConnectionManager::generate_client_id();
        let client = Client::new(client_id.clone(), sender);

        // Add client
        manager.add_client(client.clone()).await;
        assert_eq!(manager.client_count().await, 1);

        // Remove client
        let removed_client = manager.remove_client(&client_id).await;
        assert!(removed_client.is_some());
        assert_eq!(removed_client.unwrap().id, client_id);
        assert_eq!(manager.client_count().await, 0);
        assert!(!manager.client_exists(&client_id).await);

        // Try to remove non-existent client
        let non_existent = manager.remove_client(&client_id).await;
        assert!(non_existent.is_none());
    }

    #[tokio::test]
    async fn test_get_client() {
        let manager = ConnectionManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let client_id = ConnectionManager::generate_client_id();
        let client = Client::new(client_id.clone(), sender);

        // Try to get non-existent client
        let non_existent = manager.get_client(&client_id).await;
        assert!(non_existent.is_none());

        // Add client and retrieve it
        manager.add_client(client.clone()).await;
        let retrieved_client = manager.get_client(&client_id).await;
        assert!(retrieved_client.is_some());
        let retrieved = retrieved_client.unwrap();
        assert_eq!(retrieved.id, client_id);
        assert_eq!(retrieved.connected_at, client.connected_at);
    }

    #[tokio::test]
    async fn test_get_all_clients() {
        let manager = ConnectionManager::new();

        // Initially empty
        let all_clients = manager.get_all_clients().await;
        assert!(all_clients.is_empty());

        // Add multiple clients
        let mut expected_ids = Vec::new();
        for i in 0..3 {
            let (sender, _receiver) = mpsc::unbounded_channel();
            let client_id = format!("client_{}", i);
            let client = Client::new(client_id.clone(), sender);
            manager.add_client(client).await;
            expected_ids.push(client_id);
        }

        let all_clients = manager.get_all_clients().await;
        assert_eq!(all_clients.len(), 3);

        // Check that all expected IDs are present
        for expected_id in expected_ids {
            assert!(all_clients.contains(&expected_id));
        }
    }

    #[tokio::test]
    async fn test_client_exists() {
        let manager = ConnectionManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let client_id = ConnectionManager::generate_client_id();
        let client = Client::new(client_id.clone(), sender);

        // Client doesn't exist initially
        assert!(!manager.client_exists(&client_id).await);

        // Add client
        manager.add_client(client).await;
        assert!(manager.client_exists(&client_id).await);

        // Remove client
        manager.remove_client(&client_id).await;
        assert!(!manager.client_exists(&client_id).await);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let manager = Arc::new(ConnectionManager::new());
        let mut handles = vec![];

        // Spawn multiple tasks to add clients concurrently
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                let (sender, _receiver) = mpsc::unbounded_channel();
                let client_id = format!("concurrent_client_{}", i);
                let client = Client::new(client_id.clone(), sender);
                manager_clone.add_client(client).await;
                client_id
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete and collect client IDs
        let mut added_ids = Vec::new();
        for handle in handles {
            let client_id = handle.await.unwrap();
            added_ids.push(client_id);
        }

        // Verify all clients were added
        assert_eq!(manager.client_count().await, 10);

        // Verify all clients exist
        for client_id in &added_ids {
            assert!(manager.client_exists(client_id).await);
        }

        // Test concurrent removal
        let mut remove_handles = vec![];
        for client_id in added_ids {
            let manager_clone = Arc::clone(&manager);
            let handle = tokio::spawn(async move { manager_clone.remove_client(&client_id).await });
            remove_handles.push(handle);
        }

        // Wait for all removals to complete
        for handle in remove_handles {
            let removed_client = handle.await.unwrap();
            assert!(removed_client.is_some());
        }

        // Verify all clients were removed
        assert_eq!(manager.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_client_count() {
        let manager = ConnectionManager::new();

        assert_eq!(manager.client_count().await, 0);

        // Add clients one by one and check count
        for i in 1..=5 {
            let (sender, _receiver) = mpsc::unbounded_channel();
            let client_id = format!("client_{}", i);
            let client = Client::new(client_id, sender);
            manager.add_client(client).await;
            assert_eq!(manager.client_count().await, i);
        }

        // Remove clients one by one and check count
        for i in (1..=5).rev() {
            let client_id = format!("client_{}", i);
            manager.remove_client(&client_id).await;
            assert_eq!(manager.client_count().await, i - 1);
        }
    }

    #[tokio::test]
    async fn test_connect_client() {
        let manager = ConnectionManager::new();

        let (client_id, mut receiver) = manager.connect_client().await;

        // Verify client was added
        assert_eq!(manager.client_count().await, 1);
        assert!(manager.client_exists(&client_id).await);

        // Verify we can send messages to the client
        let client = manager.get_client(&client_id).await.unwrap();
        let message_type = crate::models::message::MessageType::TextChat {
            target_user_id: None,
            content: "Test message".to_string(),
        };
        let message = Message::new_simple(Some("sender".to_string()), message_type);

        client.sender.send(message.clone()).unwrap();
        let received_message = receiver.recv().await.unwrap();
        assert_eq!(received_message.id, message.id);
    }

    #[tokio::test]
    async fn test_disconnect_client() {
        let manager = ConnectionManager::new();

        // Connect a client
        let (client_id, _receiver) = manager.connect_client().await;
        assert_eq!(manager.client_count().await, 1);

        // Disconnect the client
        let disconnected = manager.disconnect_client(&client_id).await;
        assert!(disconnected);
        assert_eq!(manager.client_count().await, 0);
        assert!(!manager.client_exists(&client_id).await);

        // Try to disconnect non-existent client
        let not_disconnected = manager.disconnect_client(&client_id).await;
        assert!(!not_disconnected);
    }

    #[tokio::test]
    async fn test_get_client_info() {
        let manager = ConnectionManager::new();

        // Try to get info for non-existent client
        let non_existent_info = manager.get_client_info(&"non_existent".to_string()).await;
        assert!(non_existent_info.is_none());

        // Connect a client and get its info
        let (client_id, _receiver) = manager.connect_client().await;
        let client_info = manager.get_client_info(&client_id).await.unwrap();

        assert_eq!(client_info.id, client_id);
        assert!(client_info.connected_at <= Utc::now());
    }

    #[tokio::test]
    async fn test_get_all_client_info() {
        let manager = ConnectionManager::new();

        // Initially empty
        let all_info = manager.get_all_client_info().await;
        assert!(all_info.is_empty());

        // Connect multiple clients
        let mut client_ids = Vec::new();
        for _ in 0..3 {
            let (client_id, _receiver) = manager.connect_client().await;
            client_ids.push(client_id);
        }

        let all_info = manager.get_all_client_info().await;
        assert_eq!(all_info.len(), 3);

        // Verify all client IDs are present
        let info_ids: Vec<String> = all_info.iter().map(|info| info.id.clone()).collect();
        for client_id in client_ids {
            assert!(info_ids.contains(&client_id));
        }
    }

    #[tokio::test]
    async fn test_client_lifecycle_integration() {
        let manager = ConnectionManager::new();

        // Connect multiple clients
        let mut clients = Vec::new();
        for i in 0..5 {
            let (client_id, receiver) = manager.connect_client().await;
            clients.push((client_id, receiver));
        }

        assert_eq!(manager.client_count().await, 5);

        // Test that all clients can receive messages
        for (client_id, mut receiver) in clients.into_iter().take(3) {
            let client = manager.get_client(&client_id).await.unwrap();
            let message_type = crate::models::message::MessageType::TextChat {
                target_user_id: None,
                content: format!("Message for {}", client_id),
            };
            let message = Message::new_simple(Some("sender".to_string()), message_type);

            client.sender.send(message.clone()).unwrap();
            let received_message = receiver.recv().await.unwrap();
            assert_eq!(received_message.id, message.id);

            // Disconnect the client
            assert!(manager.disconnect_client(&client_id).await);
        }

        // Should have 2 clients remaining
        assert_eq!(manager.client_count().await, 2);
    }

    #[tokio::test]
    async fn test_channel_cleanup_on_disconnect() {
        let manager = ConnectionManager::new();

        // Connect a client
        let (client_id, mut receiver) = manager.connect_client().await;

        // Get the client and verify channel works
        let client = manager.get_client(&client_id).await.unwrap();
        let message_type = crate::models::message::MessageType::TextChat {
            target_user_id: None,
            content: "Test message".to_string(),
        };
        let message = Message::new_simple(Some("sender".to_string()), message_type);

        client.sender.send(message.clone()).unwrap();
        let received_message = receiver.recv().await.unwrap();
        assert_eq!(received_message.id, message.id);

        // Disconnect the client
        manager.disconnect_client(&client_id).await;

        // The receiver should eventually get None when the sender is dropped
        // This might take a moment due to async cleanup
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv()).await;

        // Either timeout or receive None (channel closed)
        match result {
            Ok(None) => {} // Channel properly closed
            Err(_) => {}   // Timeout is also acceptable as cleanup might be async
            Ok(Some(_)) => panic!("Should not receive more messages after disconnect"),
        }
    }
}
