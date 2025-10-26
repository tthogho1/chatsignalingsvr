use std::io::{self, Write};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use websocket_chat_signaling_server::models::message::{Message as ChatMessage, MessageType};
use tracing::{info, error, debug};
use serde_json;
use clap::{Arg, Command};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let matches = Command::new("WebSocket Chat Client")
        .version("1.0")
        .author("Your Name")
        .about("WebSocket chat client for signaling server")
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("URL")
                .help("WebSocket server URL")
                .default_value("ws://127.0.0.1:8080")
        )
        .arg(
            Arg::new("username")
                .short('u')
                .long("username")
                .value_name("NAME")
                .help("Username for chat")
                .required(false)
        )
        .get_matches();

    let server_url = matches.get_one::<String>("server").unwrap();
    let username = matches.get_one::<String>("username")
        .map(|s| s.clone())
        .unwrap_or_else(|| format!("user_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()));

    info!("Connecting to {} as {}", server_url, username);

    // Connect to WebSocket server
    let (ws_stream, _response) = connect_async(server_url).await?;
    info!("Connected to WebSocket server");

    let (mut write, mut read) = ws_stream.split();

    // Spawn task to handle incoming messages
    let read_handle = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ChatMessage>(&text) {
                        Ok(chat_msg) => {
                            match &chat_msg.message_type {
                                MessageType::TextChat { target_user_id: _, content } => {
                                    if let Some(sender) = &chat_msg.sender_id {
                                        println!("[{}]: {}", sender, content);
                                    } else {
                                        println!("[System]: {}", content);
                                    }
                                }
                                MessageType::WebRTCSignaling { target_user_id, signaling_data } => {
                                    println!("[SIGNALING to {}]: {:?}", target_user_id, signaling_data);
                                }
                                MessageType::GenericMessage { target_user_id, content } => {
                                    println!("[GENERIC to {}]: {}", target_user_id, content);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse message: {}", e);
                            println!("Raw message: {}", text);
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
    });

    // Main input loop
    println!("WebSocket Chat Client");
    println!("Commands:");
    println!("  /help                    - Show this help");
    println!("  /broadcast <message>     - Send broadcast message");
    println!("  /direct <user> <message> - Send direct message");
    println!("  /signal <user> <data>    - Send WebRTC signaling");
    println!("  /quit                    - Exit client");
    println!("  <message>                - Send broadcast message");
    println!();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                
                if input.is_empty() {
                    continue;
                }

                if input == "/quit" {
                    break;
                }

                if input == "/help" {
                    println!("Commands:");
                    println!("  /help                    - Show this help");
                    println!("  /broadcast <message>     - Send broadcast message");
                    println!("  /direct <user> <message> - Send direct message");
                    println!("  /signal <user> <data>    - Send WebRTC signaling");
                    println!("  /quit                    - Exit client");
                    println!("  <message>                - Send broadcast message");
                    continue;
                }

                let message = if input.starts_with("/broadcast ") {
                    let content = &input[11..];
                    ChatMessage::new_simple(
                        Some(username.clone()),
                        MessageType::TextChat {
                            target_user_id: None,
                            content: content.to_string(),
                        }
                    )
                } else if input.starts_with("/direct ") {
                    let parts: Vec<&str> = input[8..].splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        ChatMessage::new_simple(
                            Some(username.clone()),
                            MessageType::TextChat {
                                target_user_id: Some(parts[0].to_string()),
                                content: parts[1].to_string(),
                            }
                        )
                    } else {
                        println!("Usage: /direct <user> <message>");
                        continue;
                    }
                } else if input.starts_with("/signal ") {
                    let parts: Vec<&str> = input[8..].splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let target_user = parts[0].to_string();
                        let signaling_data = match serde_json::from_str(parts[1]) {
                            Ok(data) => data,
                            Err(_) => {
                                // Create simple signaling data if not valid JSON
                                serde_json::json!({
                                    "type": "message",
                                    "data": parts[1]
                                })
                            }
                        };
                        
                        ChatMessage::new_simple(
                            Some(username.clone()),
                            MessageType::WebRTCSignaling {
                                target_user_id: target_user,
                                signaling_data,
                            }
                        )
                    } else {
                        println!("Usage: /signal <user> <json_data>");
                        continue;
                    }
                } else {
                    // Default to broadcast
                    ChatMessage::new_simple(
                        Some(username.clone()),
                        MessageType::TextChat {
                            target_user_id: None,
                            content: input.to_string(),
                        }
                    )
                };

                // Send message
                match serde_json::to_string(&message) {
                    Ok(json) => {
                        if let Err(e) = write.send(Message::Text(json)).await {
                            error!("Failed to send message: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error reading input: {}", e);
                break;
            }
        }
    }

    info!("Closing connection...");
    let _ = write.close().await;
    let _ = read_handle.await;

    Ok(())
}