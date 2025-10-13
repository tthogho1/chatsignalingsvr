pub mod models;
pub mod handlers;
pub mod config;
pub mod logging;
pub mod client;

use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, WebSocketStream};
use tracing::{info, error, warn, debug, trace, instrument};
use futures_util::{SinkExt, StreamExt};

use crate::config::ServerConfig;
use crate::handlers::connection::ConnectionManager;
use crate::models::client::ClientRegistry;
use crate::models::error::ServerError;

/// Main WebSocket server structure
#[derive(Clone)]
pub struct WebSocketServer {
    config: ServerConfig,
    connection_manager: Arc<ConnectionManager>,
    clients: Arc<RwLock<ClientRegistry>>,
}

impl WebSocketServer {
    /// Create a new WebSocket server with the given configuration
    pub fn new(config: ServerConfig) -> Self {
        let clients = Arc::new(RwLock::new(ClientRegistry::new()));
        let connection_manager = Arc::new(ConnectionManager::new());
        
        Self {
            config,
            connection_manager,
            clients,
        }
    }

    /// Start the WebSocket server
    #[instrument(skip(self), fields(bind_address = %self.config.bind_address, port = self.config.port))]
    pub async fn start(&self) -> Result<(), ServerError> {
        let addr = self.config.socket_addr();
        info!(
            bind_address = %self.config.bind_address,
            port = self.config.port,
            max_connections = self.config.max_connections,
            "Starting WebSocket server"
        );

        // Bind TCP listener
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| {
                error!(
                    bind_address = %self.config.bind_address,
                    port = self.config.port,
                    error = %e,
                    "Failed to bind to address"
                );
                ServerError::BindError(e)
            })?;

        info!(
            bind_address = %self.config.bind_address,
            port = self.config.port,
            "WebSocket server listening successfully"
        );

        // Accept connections in a loop
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let current_connections = self.connection_manager.client_count().await;
                    debug!(
                        peer_address = %peer_addr,
                        current_connections = current_connections,
                        max_connections = self.config.max_connections,
                        "New connection attempt"
                    );
                    
                    // Check connection limit
                    if current_connections >= self.config.max_connections {
                        warn!(
                            peer_address = %peer_addr,
                            current_connections = current_connections,
                            max_connections = self.config.max_connections,
                            "Connection limit reached, rejecting connection"
                        );
                        // Close the stream immediately
                        drop(stream);
                        continue;
                    }

                    info!(
                        peer_address = %peer_addr,
                        current_connections = current_connections,
                        "Accepting new connection"
                    );

                    // Handle the connection in a separate task
                    let server_clone = self.clone_for_connection();
                    tokio::spawn(async move {
                        if let Err(e) = server_clone.handle_connection(stream).await {
                            error!(
                                peer_address = %peer_addr,
                                error = %e,
                                "Connection handling failed"
                            );
                        }
                    });
                }
                Err(e) => {
                    error!(
                        error = %e,
                        "Failed to accept incoming connection"
                    );
                    // Continue accepting other connections
                }
            }
        }
    }

    /// Handle a single WebSocket connection
    pub async fn handle_connection(&self, stream: TcpStream) -> Result<(), ServerError> {
        let connection = self.clone_for_connection();
        connection.handle_connection(stream).await
    }

    /// Create a clone of necessary components for connection handling
    fn clone_for_connection(&self) -> WebSocketServerConnection {
        WebSocketServerConnection {
            config: self.config.clone(),
            connection_manager: Arc::clone(&self.connection_manager),
            clients: Arc::clone(&self.clients),
        }
    }

    /// Get server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get connection manager
    pub fn connection_manager(&self) -> &Arc<ConnectionManager> {
        &self.connection_manager
    }

    /// Get current client count
    pub async fn client_count(&self) -> usize {
        self.connection_manager.client_count().await
    }

    /// Graceful shutdown - disconnect all clients
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<(), ServerError> {
        let client_count = self.connection_manager.client_count().await;
        info!(
            client_count = client_count,
            "Initiating WebSocket server shutdown"
        );
        
        // Get all connected client IDs
        let client_ids = self.connection_manager.get_all_clients().await;
        
        // Disconnect all clients
        let mut disconnected_count = 0;
        let mut error_count = 0;
        
        for client_id in client_ids {
            match self.disconnect_client(&client_id).await {
                Ok(()) => {
                    disconnected_count += 1;
                    debug!(
                        client_id = %client_id,
                        "Client disconnected during shutdown"
                    );
                }
                Err(e) => {
                    error_count += 1;
                    warn!(
                        client_id = %client_id,
                        error = %e,
                        "Error disconnecting client during shutdown"
                    );
                }
            }
        }
        
        info!(
            disconnected_clients = disconnected_count,
            error_count = error_count,
            "WebSocket server shutdown complete"
        );
        Ok(())
    }

    /// Disconnect a specific client
    #[instrument(skip(self), fields(client_id = %client_id))]
    async fn disconnect_client(&self, client_id: &str) -> Result<(), ServerError> {
        let client_id_string = client_id.to_string();
        if self.connection_manager.disconnect_client(&client_id_string).await {
            debug!(
                client_id = %client_id,
                "Client disconnected successfully"
            );
            Ok(())
        } else {
            warn!(
                client_id = %client_id,
                "Client not found during disconnect attempt"
            );
            Ok(())
        }
    }
}

/// Helper struct for handling individual connections
#[derive(Clone)]
struct WebSocketServerConnection {
    config: ServerConfig,
    connection_manager: Arc<ConnectionManager>,
    clients: Arc<RwLock<ClientRegistry>>,
}

impl WebSocketServerConnection {
    /// Handle a single WebSocket connection
    #[instrument(skip(self, stream))]
    async fn handle_connection(&self, stream: TcpStream) -> Result<(), ServerError> {
        // Perform WebSocket upgrade
        let ws_stream = match accept_async(stream).await {
            Ok(ws) => {
                debug!("WebSocket upgrade successful");
                ws
            }
            Err(e) => {
                error!(
                    error = %e,
                    "WebSocket upgrade failed"
                );
                return Err(ServerError::ConnectionError(
                    crate::models::error::ConnectionError::WebSocketError(e)
                ));
            }
        };

        // Connect the client and get their ID and message receiver
        let (client_id, message_receiver) = self.connection_manager.connect_client().await;
        
        // Also add the client to the shared registry for message handlers
        if let Some(client) = self.connection_manager.get_client(&client_id).await {
            self.clients.write().await.insert(client_id.clone(), client);
        }
        
        let current_connections = self.connection_manager.client_count().await;
        info!(
            client_id = %client_id,
            total_connections = current_connections,
            "Client connected successfully"
        );

        // Split the WebSocket stream for concurrent reading and writing
        let (ws_sender, ws_receiver) = ws_stream.split();

        // Handle the client connection with message processing
        let result = self.handle_client_loop(client_id.clone(), ws_sender, ws_receiver, message_receiver).await;

        // Clean up: disconnect the client from both registries
        self.connection_manager.disconnect_client(&client_id).await;
        self.clients.write().await.remove(&client_id);
        
        let remaining_connections = self.connection_manager.client_count().await;
        info!(
            client_id = %client_id,
            total_connections = remaining_connections,
            "Client disconnected"
        );

        result
    }

    /// Main client connection loop handling both incoming and outgoing messages
    #[instrument(skip(self, ws_sender, ws_receiver, message_receiver), fields(client_id = %client_id))]
    async fn handle_client_loop(
        &self,
        client_id: String,
        mut ws_sender: futures_util::stream::SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
        mut ws_receiver: futures_util::stream::SplitStream<WebSocketStream<TcpStream>>,
        mut message_receiver: tokio::sync::mpsc::UnboundedReceiver<crate::models::message::Message>,
    ) -> Result<(), ServerError> {

        loop {
            tokio::select! {
                // Handle incoming WebSocket messages from client
                ws_msg = ws_receiver.next() => {
                    match ws_msg {
                        Some(Ok(msg)) => {
                            trace!(
                                client_id = %client_id,
                                message_type = ?msg,
                                "Received WebSocket message from client"
                            );
                            
                            if let Err(e) = self.process_incoming_message(&client_id, msg).await {
                                warn!(
                                    client_id = %client_id,
                                    error = %e,
                                    "Error processing message from client"
                                );
                                // Send error response to client
                                if let Err(send_err) = self.send_error_response(&mut ws_sender, &e).await {
                                    error!(
                                        client_id = %client_id,
                                        error = %send_err,
                                        "Failed to send error response to client"
                                    );
                                    break;
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!(
                                client_id = %client_id,
                                error = %e,
                                "WebSocket error for client"
                            );
                            break;
                        }
                        None => {
                            info!(
                                client_id = %client_id,
                                "WebSocket connection closed by client"
                            );
                            break;
                        }
                    }
                }

                // Handle outgoing messages to client
                app_msg = message_receiver.recv() => {
                    match app_msg {
                        Some(msg) => {
                            trace!(
                                client_id = %client_id,
                                message_id = %msg.id,
                                sender_id = ?msg.sender_id,
                                "Sending message to client"
                            );
                            
                            if let Err(e) = self.send_message_to_client(&mut ws_sender, msg).await {
                                error!(
                                    client_id = %client_id,
                                    error = %e,
                                    "Failed to send message to client"
                                );
                                break;
                            }
                        }
                        None => {
                            debug!(
                                client_id = %client_id,
                                "Message channel closed for client"
                            );
                            break;
                        }
                    }
                }
            }
        }

        // Close the WebSocket connection gracefully
        if let Err(e) = ws_sender.close().await {
            warn!(
                client_id = %client_id,
                error = %e,
                "Error closing WebSocket connection"
            );
        } else {
            debug!(
                client_id = %client_id,
                "WebSocket connection closed gracefully"
            );
        }

        Ok(())
    }

    /// Process incoming WebSocket message from client
    #[instrument(skip(self, ws_message), fields(client_id = %client_id))]
    async fn process_incoming_message(
        &self,
        client_id: &str,
        ws_message: tokio_tungstenite::tungstenite::Message,
    ) -> Result<(), crate::models::error::ConnectionError> {
        use tokio_tungstenite::tungstenite::Message as WsMessage;

        match ws_message {
            WsMessage::Text(text) => {
                debug!(
                    client_id = %client_id,
                    message_length = text.len(),
                    "Processing text message from client"
                );
                
                // Parse JSON message
                let parsed_message: crate::models::message::Message = serde_json::from_str(&text)
                    .map_err(|e| {
                        warn!(
                            client_id = %client_id,
                            error = %e,
                            message_preview = %text.chars().take(100).collect::<String>(),
                            "Failed to parse JSON message"
                        );
                        crate::models::error::ConnectionError::InvalidMessage(
                            format!("Failed to parse JSON message: {}", e)
                        )
                    })?;

                debug!(
                    client_id = %client_id,
                    message_id = %parsed_message.id,
                    message_type = ?parsed_message.message_type,
                    "Successfully parsed message, routing to handler"
                );

                self.route_message(client_id, parsed_message).await
            }
            WsMessage::Close(_) => {
                info!(
                    client_id = %client_id,
                    "Client sent close frame"
                );
                Ok(())
            }
            WsMessage::Ping(_) => {
                debug!(
                    client_id = %client_id,
                    "Received ping from client"
                );
                // Pong will be sent automatically by tokio-tungstenite
                Ok(())
            }
            WsMessage::Pong(_) => {
                trace!(
                    client_id = %client_id,
                    "Received pong from client"
                );
                Ok(())
            }
            WsMessage::Binary(_) => {
                warn!(
                    client_id = %client_id,
                    "Client sent unsupported binary message"
                );
                Err(crate::models::error::ConnectionError::InvalidMessage(
                    "Binary messages are not supported".to_string()
                ))
            }
            WsMessage::Frame(_) => {
                trace!(
                    client_id = %client_id,
                    "Received raw frame from client"
                );
                Ok(())
            }
        }
    }

    /// Route parsed message to appropriate handler
    #[instrument(skip(self, message), fields(client_id = %client_id, message_id = %message.id))]
    async fn route_message(
        &self,
        client_id: &str,
        message: crate::models::message::Message,
    ) -> Result<(), crate::models::error::ConnectionError> {
        use crate::models::message::MessageType;
        use crate::handlers::{message::MessageHandler, signaling::SignalingHandler};

        // Create handlers with shared client registry
        let message_handler = MessageHandler::new(Arc::clone(&self.clients));
        let signaling_handler = SignalingHandler::new(Arc::clone(&self.clients));

        // Route based on message type
        let result = match &message.message_type {
            MessageType::TextChat { target_user_id, content } => {
                info!(
                    client_id = %client_id,
                    message_id = %message.id,
                    target_user_id = ?target_user_id,
                    content_length = content.len(),
                    message_type = "text_chat",
                    "Routing text chat message"
                );
                
                message_handler.handle_text_chat(
                    client_id.to_string(),
                    target_user_id.clone(),
                    content.clone(),
                ).await
            }
            MessageType::WebRTCSignaling { target_user_id, signaling_data } => {
                info!(
                    client_id = %client_id,
                    message_id = %message.id,
                    target_user_id = %target_user_id,
                    signaling_type = ?signaling_data.get("type"),
                    message_type = "webrtc_signaling",
                    "Routing WebRTC signaling message"
                );
                
                signaling_handler.handle_webrtc_signaling(
                    client_id.to_string(),
                    target_user_id.clone(),
                    signaling_data.clone(),
                ).await
            }
            MessageType::GenericMessage { target_user_id, content } => {
                info!(
                    client_id = %client_id,
                    message_id = %message.id,
                    target_user_id = %target_user_id,
                    content_length = content.len(),
                    message_type = "generic_message",
                    "Routing generic message"
                );
                
                message_handler.handle_generic_message(
                    client_id.to_string(),
                    target_user_id.clone(),
                    content.clone(),
                ).await
            }
        };

        match &result {
            Ok(()) => {
                debug!(
                    client_id = %client_id,
                    message_id = %message.id,
                    "Message routed successfully"
                );
            }
            Err(e) => {
                error!(
                    client_id = %client_id,
                    message_id = %message.id,
                    error = %e,
                    "Message routing failed"
                );
            }
        }

        result
    }

    /// Send a message to the WebSocket client
    async fn send_message_to_client(
        &self,
        ws_sender: &mut futures_util::stream::SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
        message: crate::models::message::Message,
    ) -> Result<(), crate::models::error::ConnectionError> {
        use futures_util::SinkExt;
        use tokio_tungstenite::tungstenite::Message as WsMessage;

        // Serialize message to JSON
        let json_text = serde_json::to_string(&message)
            .map_err(|e| crate::models::error::ConnectionError::InvalidMessage(
                format!("Failed to serialize message: {}", e)
            ))?;

        // Send as WebSocket text message
        ws_sender.send(WsMessage::Text(json_text)).await
            .map_err(|e| crate::models::error::ConnectionError::WebSocketError(e))?;

        Ok(())
    }

    /// Send error response to client
    async fn send_error_response(
        &self,
        ws_sender: &mut futures_util::stream::SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
        error: &crate::models::error::ConnectionError,
    ) -> Result<(), crate::models::error::ConnectionError> {
        use crate::models::message::{Message, MessageType};

        // Create error message
        let error_message = Message::new(
            None, // System message, no sender
            MessageType::TextChat {
                target_user_id: None,
                content: format!("Error: {}", error),
            }
        );

        self.send_message_to_client(ws_sender, error_message).await
    }
}