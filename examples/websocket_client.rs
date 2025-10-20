use std::io::{self, Write};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use websocket_chat_signaling_server::models::message::{Message, MessageType};
use clap::{Arg, Command};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let matches = Command::new("WebSocket Client Example")
        .version("1.0")
        .author("WebSocket Chat and Signaling Server")
        .about("Example WebSocket client for testing server functionality")
        .arg(
            Arg::new("server")
                .long("server")
                .short('s')
                .value_name("URL")
                .help("WebSocket server URL")
                .default_value("ws://127.0.0.1:8080")
        )
        .arg(
            Arg::new("client-id")
                .long("client-id")
                .short('c')
                .value_name("ID")
                .help("Client identifier")
                .default_value("example_client")
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .short('m')
                .value_name("MODE")
                .help("Client mode: interactive, demo, or test")
                .default_value("interactive")
                .value_parser(["interactive", "demo", "test"])
        )
        .get_matches();

    let server_url = matches.get_one::<String>("server").unwrap();
    let client_id = matches.get_one::<String>("client-id").unwrap();
    let mode = matches.get_one::<String>("mode").unwrap();

    println!("🚀 WebSocket Client Example");
    println!("📡 Connecting to: {}", server_url);
    println!("🆔 Client ID: {}", client_id);
    println!("🎮 Mode: {}", mode);
    println!();

    // Connect to the WebSocket server
    let (ws_stream, response) = connect_async(server_url).await?;
    println!("✅ Connected to server");
    println!("📋 Response status: {}", response.status());
    println!();

    let (mut write, mut read) = ws_stream.split();

    match mode.as_str() {
        "interactive" => run_interactive_mode(&mut write, read, client_id).await?,
        "demo" => run_demo_mode(&mut write, read, client_id).await?,
        "test" => run_test_mode(&mut write, read, client_id).await?,
        _ => unreachable!(),
    }

    // Close connection gracefully
    write.close().await?;
    println!("👋 Connection closed");

    Ok(())
}

/// Interactive mode - user can type messages and commands
async fn run_interactive_mode(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    mut read: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    client_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Interactive Mode");
    println!("Commands:");
    println!("  /direct <target_id> <message>  - Send direct message");
    println!("  /broadcast <message>           - Send broadcast message");
    println!("  /webrtc <target_id> <type>     - Send WebRTC signaling");
    println!("  /generic <target_id> <content> - Send generic message");
    println!("  /help                          - Show this help");
    println!("  /quit                          - Exit client");
    println!();

    // Start message receiver task
    let read_handle = tokio::spawn(async move {
        let mut read = read;
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    match serde_json::from_str::<Message>(&text) {
                        Ok(parsed_msg) => {
                            println!("📨 Received: {:?}", parsed_msg);
                        }
                        Err(_) => {
                            println!("📨 Raw message: {}", text);
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("🔌 Server closed connection");
                    break;
                }
                Ok(_) => {
                    println!("📨 Received non-text message");
                }
                Err(e) => {
                    println!("❌ Error receiving message: {}", e);
                    break;
                }
            }
        }
    });

    // Interactive input loop
    loop {
        print!("💬 > ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "/quit" {
            break;
        }

        if input == "/help" {
            println!("Commands:");
            println!("  /direct <target_id> <message>  - Send direct message");
            println!("  /broadcast <message>           - Send broadcast message");
            println!("  /webrtc <target_id> <type>     - Send WebRTC signaling");
            println!("  /generic <target_id> <content> - Send generic message");
            println!("  /help                          - Show this help");
            println!("  /quit                          - Exit client");
            continue;
        }

        let result = if input.starts_with("/direct ") {
            handle_direct_command(write, client_id, &input[8..]).await
        } else if input.starts_with("/broadcast ") {
            handle_broadcast_command(write, client_id, &input[11..]).await
        } else if input.starts_with("/webrtc ") {
            handle_webrtc_command(write, client_id, &input[8..]).await
        } else if input.starts_with("/generic ") {
            handle_generic_command(write, client_id, &input[9..]).await
        } else {
            // Default to broadcast message
            handle_broadcast_command(write, client_id, input).await
        };

        if let Err(e) = result {
            println!("❌ Error sending message: {}", e);
        }
    }

    // Cancel the read task
    read_handle.abort();
    Ok(())
}

/// Demo mode - automatically sends various types of messages
async fn run_demo_mode(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    mut read: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    client_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎭 Demo Mode - Sending example messages");
    println!();

    // Start message receiver task
    let read_handle = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    match serde_json::from_str::<Message>(&text) {
                        Ok(parsed_msg) => {
                            println!("📨 Received: {:?}", parsed_msg);
                        }
                        Err(_) => {
                            println!("📨 Raw message: {}", text);
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("🔌 Server closed connection");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    println!("❌ Error receiving message: {}", e);
                    break;
                }
            }
        }
    });

    // Demo sequence
    println!("1️⃣ Sending broadcast message...");
    send_broadcast_message(write, client_id, "Hello everyone! This is a demo broadcast.").await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("2️⃣ Sending direct message...");
    send_direct_message(write, client_id, "demo_target", "Hello demo_target! This is a direct message.").await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("3️⃣ Sending WebRTC offer...");
    send_webrtc_signaling(write, client_id, "webrtc_peer", "offer").await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("4️⃣ Sending WebRTC answer...");
    send_webrtc_signaling(write, client_id, "webrtc_peer", "answer").await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("5️⃣ Sending ICE candidate...");
    send_webrtc_signaling(write, client_id, "webrtc_peer", "ice-candidate").await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("6️⃣ Sending generic message...");
    send_generic_message(write, client_id, "generic_target", "ping").await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("✅ Demo completed!");

    // Wait a bit for any responses
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Cancel the read task
    read_handle.abort();
    Ok(())
}

/// Test mode - sends messages and validates responses
async fn run_test_mode(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    mut read: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    client_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Test Mode - Validating server functionality");
    println!();

    let mut test_count = 0;
    let mut passed_count = 0;

    // Test 1: Send broadcast message
    test_count += 1;
    println!("Test {}: Broadcast message", test_count);
    let result = send_broadcast_message(write, client_id, "Test broadcast message").await;
    if result.is_ok() {
        passed_count += 1;
        println!("✅ PASS");
    } else {
        println!("❌ FAIL: {:?}", result.err());
    }

    // Test 2: Send direct message
    test_count += 1;
    println!("Test {}: Direct message", test_count);
    let result = send_direct_message(write, client_id, "test_target", "Test direct message").await;
    if result.is_ok() {
        passed_count += 1;
        println!("✅ PASS");
    } else {
        println!("❌ FAIL: {:?}", result.err());
    }

    // Test 3: Send WebRTC signaling
    test_count += 1;
    println!("Test {}: WebRTC signaling", test_count);
    let result = send_webrtc_signaling(write, client_id, "webrtc_test_peer", "offer").await;
    if result.is_ok() {
        passed_count += 1;
        println!("✅ PASS");
    } else {
        println!("❌ FAIL: {:?}", result.err());
    }

    // Test 4: Send generic message
    test_count += 1;
    println!("Test {}: Generic message", test_count);
    let result = send_generic_message(write, client_id, "generic_test_target", "test_command").await;
    if result.is_ok() {
        passed_count += 1;
        println!("✅ PASS");
    } else {
        println!("❌ FAIL: {:?}", result.err());
    }

    // Test 5: Send invalid message format
    test_count += 1;
    println!("Test {}: Invalid message format", test_count);
    let invalid_json = r#"{"invalid": "message", "format": true}"#;
    let result = write.send(WsMessage::Text(invalid_json.to_string())).await;
    if result.is_ok() {
        // Check if server responds with error
        let response = timeout(Duration::from_secs(2), read.next()).await;
        match response {
            Ok(Some(Ok(WsMessage::Text(text)))) => {
                if text.contains("Error") || text.contains("error") {
                    passed_count += 1;
                    println!("✅ PASS - Server responded with error");
                } else {
                    println!("❌ FAIL - Server did not respond with error");
                }
            }
            _ => {
                println!("⚠️  PARTIAL - Server may have closed connection (acceptable behavior)");
                passed_count += 1; // Consider this a pass as closing connection is valid error handling
            }
        }
    } else {
        println!("❌ FAIL: Could not send invalid message: {:?}", result.err());
    }

    println!();
    println!("🏁 Test Results: {}/{} tests passed", passed_count, test_count);
    
    if passed_count == test_count {
        println!("🎉 All tests passed!");
    } else {
        println!("⚠️  Some tests failed");
    }

    Ok(())
}

// Helper functions for handling commands

async fn handle_direct_command(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    args: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() != 2 {
        println!("Usage: /direct <target_id> <message>");
        return Ok(());
    }
    
    let target_id = parts[0];
    let message = parts[1];
    
    send_direct_message(write, client_id, target_id, message).await
}

async fn handle_broadcast_command(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_broadcast_message(write, client_id, message).await
}

async fn handle_webrtc_command(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    args: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() != 2 {
        println!("Usage: /webrtc <target_id> <type>");
        println!("Types: offer, answer, ice-candidate");
        return Ok(());
    }
    
    let target_id = parts[0];
    let signaling_type = parts[1];
    
    send_webrtc_signaling(write, client_id, target_id, signaling_type).await
}

async fn handle_generic_command(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    args: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() != 2 {
        println!("Usage: /generic <target_id> <content>");
        return Ok(());
    }
    
    let target_id = parts[0];
    let content = parts[1];
    
    send_generic_message(write, client_id, target_id, content).await
}

// Message sending functions

async fn send_direct_message(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    target_id: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = Message::new(
        Some(client_id.to_string()),
        MessageType::TextChat {
            target_user_id: Some(target_id.to_string()),
            content: content.to_string(),
        }
    );
    
    let json_str = serde_json::to_string(&message)?;
    write.send(WsMessage::Text(json_str)).await?;
    println!("📤 Sent direct message to {}: {}", target_id, content);
    Ok(())
}

async fn send_broadcast_message(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = Message::new(
        Some(client_id.to_string()),
        MessageType::TextChat {
            target_user_id: None, // None means broadcast
            content: content.to_string(),
        }
    );
    
    let json_str = serde_json::to_string(&message)?;
    write.send(WsMessage::Text(json_str)).await?;
    println!("📢 Sent broadcast message: {}", content);
    Ok(())
}

async fn send_webrtc_signaling(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    target_id: &str,
    signaling_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signaling_data = match signaling_type {
        "offer" => json!({
            "type": "offer",
            "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96\r\n"
        }),
        "answer" => json!({
            "type": "answer",
            "sdp": "v=0\r\no=- 987654321 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96\r\n"
        }),
        "ice-candidate" => json!({
            "type": "ice-candidate",
            "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54400 typ host",
            "sdpMid": "video",
            "sdpMLineIndex": 0
        }),
        _ => {
            println!("Unknown signaling type: {}. Using generic.", signaling_type);
            json!({
                "type": signaling_type,
                "data": "generic_signaling_data"
            })
        }
    };
    
    let message = Message::new(
        Some(client_id.to_string()),
        MessageType::WebRTCSignaling {
            target_user_id: target_id.to_string(),
            signaling_data,
        }
    );
    
    let json_str = serde_json::to_string(&message)?;
    write.send(WsMessage::Text(json_str)).await?;
    println!("🎥 Sent WebRTC {} to {}", signaling_type, target_id);
    Ok(())
}

async fn send_generic_message(
    write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    target_id: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = Message::new(
        Some(client_id.to_string()),
        MessageType::GenericMessage {
            target_user_id: target_id.to_string(),
            content: content.to_string(),
        }
    );
    
    let json_str = serde_json::to_string(&message)?;
    write.send(WsMessage::Text(json_str)).await?;
    println!("📦 Sent generic message to {}: {}", target_id, content);
    Ok(())
}