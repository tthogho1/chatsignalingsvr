use websocket_chat_signaling_server::{WebSocketServer, config::ServerConfig, logging};
use tracing::{info, error};
use clap::{Arg, Command};
use std::env;
use std::net::IpAddr;
use std::str::FromStr;
use tokio::signal;

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let matches = build_cli().get_matches();

    // Load base configuration from environment
    let mut config = match ServerConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Override configuration with CLI arguments if provided
    if let Some(bind_address) = matches.get_one::<String>("bind-address") {
        match IpAddr::from_str(bind_address) {
            Ok(addr) => config.bind_address = addr,
            Err(e) => {
                eprintln!("Invalid bind address '{}': {}", bind_address, e);
                std::process::exit(1);
            }
        }
    }

    if let Some(port) = matches.get_one::<String>("port") {
        match port.parse::<u16>() {
            Ok(p) if p > 0 => config.port = p,
            Ok(_) => {
                eprintln!("Port cannot be 0");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Invalid port '{}': {}", port, e);
                std::process::exit(1);
            }
        }
    }

    if let Some(max_connections) = matches.get_one::<String>("max-connections") {
        match max_connections.parse::<usize>() {
            Ok(max) if max > 0 => config.max_connections = max,
            Ok(_) => {
                eprintln!("Max connections cannot be 0");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Invalid max connections '{}': {}", max_connections, e);
                std::process::exit(1);
            }
        }
    }

    if let Some(log_level) = matches.get_one::<String>("log-level") {
        match log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {
                config.log_level = log_level.to_lowercase();
            }
            _ => {
                eprintln!("Invalid log level '{}'. Valid levels: trace, debug, info, warn, error", log_level);
                std::process::exit(1);
            }
        }
    }

    // Initialize structured logging
    if let Err(e) = logging::init_logging(&config) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    info!("WebSocket Chat and Signaling Server starting up");
    info!(
        version = env!("CARGO_PKG_VERSION"),
        name = env!("CARGO_PKG_NAME"),
        bind_address = %config.bind_address,
        port = config.port,
        max_connections = config.max_connections,
        log_level = %config.log_level,
        "Application configuration"
    );

    // Create the WebSocket server
    let server = WebSocketServer::new(config);
    
    info!("Server initialized, starting to accept connections");
    
    // Set up graceful shutdown handling
    let server_handle = tokio::spawn({
        let server = server.clone();
        async move {
            if let Err(e) = server.start().await {
                error!(
                    error = %e,
                    "Server failed to start"
                );
                std::process::exit(1);
            }
        }
    });

    // Wait for shutdown signal
    let shutdown_signal = wait_for_shutdown_signal();
    
    tokio::select! {
        _ = server_handle => {
            error!("Server task completed unexpectedly");
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, initiating graceful shutdown");
            
            // Perform graceful shutdown
            if let Err(e) = server.shutdown().await {
                error!(
                    error = %e,
                    "Error during server shutdown"
                );
            } else {
                info!("Server shutdown completed successfully");
            }
        }
    }
}

/// Build the command line interface
fn build_cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("WebSocket Chat and Signaling Server")
        .about("A unified WebSocket server for chat messaging and WebRTC signaling")
        .long_about(
            "WebSocket Chat and Signaling Server provides a unified WebSocket endpoint \
            for both text-based chat messaging and WebRTC signaling. The server manages \
            client connections in-memory and supports direct messaging to specific users \
            and broadcast messaging to all connected clients."
        )
        .arg(
            Arg::new("bind-address")
                .long("bind-address")
                .short('b')
                .value_name("ADDRESS")
                .help("IP address to bind the server to")
                .long_help(
                    "The IP address that the server will bind to for accepting connections. \
                    Use 0.0.0.0 to bind to all interfaces, or 127.0.0.1 for localhost only. \
                    Can also be set via SERVER_BIND_ADDRESS environment variable."
                )
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_name("PORT")
                .help("Port number to listen on")
                .long_help(
                    "The port number that the server will listen on for WebSocket connections. \
                    Must be a valid port number between 1 and 65535. \
                    Can also be set via SERVER_PORT environment variable."
                )
        )
        .arg(
            Arg::new("max-connections")
                .long("max-connections")
                .short('m')
                .value_name("COUNT")
                .help("Maximum number of concurrent connections")
                .long_help(
                    "The maximum number of concurrent WebSocket connections the server will accept. \
                    When this limit is reached, new connections will be rejected. \
                    Can also be set via MAX_CONNECTIONS environment variable."
                )
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .short('l')
                .value_name("LEVEL")
                .help("Set the logging level")
                .long_help(
                    "Set the logging verbosity level. Valid levels are: trace, debug, info, warn, error. \
                    Higher levels include all lower level messages. \
                    Can also be set via LOG_LEVEL environment variable."
                )
                .value_parser(["trace", "debug", "info", "warn", "error"])
        )
        .after_help(
            "EXAMPLES:\n    \
            websocket-chat-signaling-server --bind-address 0.0.0.0 --port 8080\n    \
            websocket-chat-signaling-server --max-connections 500 --log-level debug\n    \
            websocket-chat-signaling-server --help\n\n\
            ENVIRONMENT VARIABLES:\n    \
            SERVER_BIND_ADDRESS    IP address to bind to (default: 127.0.0.1)\n    \
            SERVER_PORT           Port to listen on (default: 8080)\n    \
            MAX_CONNECTIONS       Maximum concurrent connections (default: 1000)\n    \
            LOG_LEVEL            Logging level (default: info)\n\n\
            SIGNALS:\n    \
            SIGINT (Ctrl+C)       Initiate graceful shutdown\n    \
            SIGTERM              Initiate graceful shutdown"
        )
}

/// Wait for shutdown signals (SIGINT, SIGTERM)
async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C)");
        },
        _ = terminate => {
            info!("Received SIGTERM");
        },
    }
}