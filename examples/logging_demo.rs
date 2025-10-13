use websocket_chat_signaling_server::{config::ServerConfig, logging};
use tracing::{info, debug, warn, error, trace};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set environment variables for demo
    env::set_var("LOG_LEVEL", "debug");
    env::set_var("SERVER_BIND_ADDRESS", "127.0.0.1");
    env::set_var("SERVER_PORT", "8080");
    env::set_var("MAX_CONNECTIONS", "100");

    // Load configuration
    let config = ServerConfig::from_env()?;
    
    // Initialize structured logging
    logging::init_logging(&config)?;

    // Demonstrate different log levels and structured logging
    info!(
        demo_name = "logging_demo",
        version = "1.0.0",
        "Starting logging demonstration"
    );

    // Simulate server startup logging
    info!(
        bind_address = %config.bind_address,
        port = config.port,
        max_connections = config.max_connections,
        log_level = %config.log_level,
        "Server configuration loaded"
    );

    // Simulate client connection logging
    let client_id = "demo-client-123";
    let peer_addr = "192.168.1.100:54321";
    
    debug!(
        client_id = %client_id,
        peer_address = %peer_addr,
        current_connections = 1,
        "New client connection"
    );

    info!(
        client_id = %client_id,
        total_connections = 1,
        "Client connected successfully"
    );

    // Simulate message routing logging
    let message_id = "msg-456";
    let sender_id = "client-123";
    let target_id = "client-789";

    info!(
        client_id = %sender_id,
        message_id = %message_id,
        target_user_id = %target_id,
        content_length = 25,
        message_type = "text_chat",
        "Routing text chat message"
    );

    debug!(
        sender_id = %sender_id,
        target_user_id = %target_id,
        "Routing direct text message"
    );

    info!(
        sender_id = %sender_id,
        target_user_id = %target_id,
        "Text chat message delivered successfully"
    );

    // Simulate WebRTC signaling logging
    info!(
        client_id = %sender_id,
        message_id = "msg-789",
        target_user_id = %target_id,
        signaling_type = "offer",
        message_type = "webrtc_signaling",
        "Routing WebRTC signaling message"
    );

    // Simulate broadcast message logging
    let broadcast_recipients = 5;
    info!(
        sender_id = %sender_id,
        recipient_count = broadcast_recipients,
        "Broadcasting text message"
    );

    info!(
        sender_id = %sender_id,
        successful_deliveries = broadcast_recipients,
        failed_deliveries = 0,
        "Broadcast message delivery completed"
    );

    // Simulate error logging
    warn!(
        client_id = "problematic-client",
        error = "Connection timeout",
        "Client connection issue detected"
    );

    error!(
        client_id = "failed-client",
        error = "Invalid message format",
        message_preview = "{\"invalid\": json",
        "Message parsing failed"
    );

    // Simulate client disconnection
    let connection_duration_seconds = 300; // 5 minutes
    info!(
        client_id = %client_id,
        connection_duration_seconds = connection_duration_seconds,
        total_connections = 0,
        "Client disconnected"
    );

    // Simulate server shutdown
    info!(
        disconnected_clients = 1,
        error_count = 0,
        "WebSocket server shutdown complete"
    );

    println!("\nLogging demonstration completed!");
    println!("Check the console output above to see structured JSON logging in action.");
    println!("In a real deployment, these logs would be captured by your logging infrastructure.");

    Ok(())
}