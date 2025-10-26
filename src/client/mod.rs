use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
    MaybeTlsStream,
    WebSocketStream as TokioWebSocketStream,
};
use tokio::net::TcpStream;
use crate::models::message::{Message as ChatMessage, MessageType};
use tracing::{info, error, debug};
use serde_json;
use std::error::Error;
use tokio::sync::mpsc;

type Socket = TokioWebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WebSocketSink = futures_util::stream::SplitSink<Socket, Message>;
pub type WebSocketStream = futures_util::stream::SplitStream<Socket>;

/// WebSocket client for chat signaling server
pub struct ChatClient {
    username: String,
    write: WebSocketSink,
    message_sender: mpsc::UnboundedSender<ChatMessage>,
}

impl ChatClient {
    /// Connect to the WebSocket server
    pub async fn connect(server_url: &str, username: String) -> Result<(Self, mpsc::UnboundedReceiver<ChatMessage>), Box<dyn Error>> {
        info!("Connecting to {} as {}", server_url, username);
        
        let (ws_stream, _response) = connect_async(server_url).await?;
        info!("Connected to WebSocket server");

        let (write, read) = ws_stream.split();
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        // Spawn task to handle incoming messages
        let sender_clone = message_sender.clone();
        tokio::spawn(async move {
            Self::handle_incoming_messages(read, sender_clone).await;
        });

        let client = ChatClient {
            username,
            write,
            message_sender,
        };

        Ok((client, message_receiver))
    }

    /// Handle incoming WebSocket messages
    async fn handle_incoming_messages(
        mut read: WebSocketStream,
        message_sender: mpsc::UnboundedSender<ChatMessage>,
    ) {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ChatMessage>(&text) {
                        Ok(chat_msg) => {
                            if let Err(e) = message_sender.send(chat_msg) {
                                error!("Failed to forward message: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse message: {}", e);
                            debug!("Raw message: {}", text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Server closed connection");
                    break;
                }
                Ok(_) => {
                    debug!("Received non-text message");
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    }

    /// Send a broadcast text message (target_user_id = None)
    pub async fn send_broadcast(&mut self, content: &str) -> Result<(), Box<dyn Error>> {
        let message = ChatMessage::new_simple(
            Some(self.username.clone()),
            MessageType::TextChat {
                target_user_id: None,
                content: content.to_string(),
            }
        );
        self.send_message(message).await
    }

    /// Send a direct text message to a specific user
    pub async fn send_direct_message(&mut self, target_user_id: &str, content: &str) -> Result<(), Box<dyn Error>> {
        let message = ChatMessage::new_simple(
            Some(self.username.clone()),
            MessageType::TextChat {
                target_user_id: Some(target_user_id.to_string()),
                content: content.to_string(),
            }
        );
        self.send_message(message).await
    }

    /// Send WebRTC signaling data
    pub async fn send_signaling(
        &mut self, 
        target_user_id: &str, 
        signaling_data: serde_json::Value
    ) -> Result<(), Box<dyn Error>> {
        let message = ChatMessage::new_simple(
            Some(self.username.clone()),
            MessageType::WebRTCSignaling {
                target_user_id: target_user_id.to_string(),
                signaling_data,
            }
        );
        self.send_message(message).await
    }

    /// Send a generic message to a specific user (arbitrary content as string)
    pub async fn send_generic_message(&mut self, target_user_id: &str, content: &str) -> Result<(), Box<dyn Error>> {
        let message = ChatMessage::new_simple(
            Some(self.username.clone()),
            MessageType::GenericMessage {
                target_user_id: target_user_id.to_string(),
                content: content.to_string(),
            }
        );
        self.send_message(message).await
    }

    /// Send a chat message
    async fn send_message(&mut self, message: ChatMessage) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(&message)?;
        self.write.send(Message::Text(json)).await?;
        Ok(())
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<(), Box<dyn Error>> {
        self.write.close().await?;
        Ok(())
    }

    /// Get the username
    pub fn username(&self) -> &str {
        &self.username
    }
}