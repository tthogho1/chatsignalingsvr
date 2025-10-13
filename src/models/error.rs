use thiserror::Error;

use super::message::ClientId;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed to bind to address: {0}")]
    BindError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Connection error: {0}")]
    ConnectionError(#[from] ConnectionError),
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Client not found: {0}")]
    ClientNotFound(ClientId),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::error::Error;

    #[test]
    fn test_server_error_display() {
        let io_error = io::Error::new(io::ErrorKind::AddrInUse, "Address already in use");
        let server_error = ServerError::BindError(io_error);
        let error_string = format!("{}", server_error);
        assert!(error_string.contains("Failed to bind to address"));
        assert!(error_string.contains("Address already in use"));
    }

    #[test]
    fn test_server_error_config_error() {
        let config_error = ServerError::ConfigError("Invalid port number".to_string());
        let error_string = format!("{}", config_error);
        assert_eq!(error_string, "Configuration error: Invalid port number");
    }

    #[test]
    fn test_server_error_from_connection_error() {
        let connection_error = ConnectionError::ClientNotFound("client123".to_string());
        let server_error = ServerError::ConnectionError(connection_error);
        let error_string = format!("{}", server_error);
        assert!(error_string.contains("Connection error"));
        assert!(error_string.contains("Client not found: client123"));
    }

    #[test]
    fn test_connection_error_client_not_found() {
        let error = ConnectionError::ClientNotFound("missing_client".to_string());
        let error_string = format!("{}", error);
        assert_eq!(error_string, "Client not found: missing_client");
    }

    #[test]
    fn test_connection_error_invalid_message() {
        let error = ConnectionError::InvalidMessage("Malformed JSON".to_string());
        let error_string = format!("{}", error);
        assert_eq!(error_string, "Invalid message format: Malformed JSON");
    }

    #[test]
    fn test_connection_error_delivery_failed() {
        let error = ConnectionError::DeliveryFailed("Channel closed".to_string());
        let error_string = format!("{}", error);
        assert_eq!(error_string, "Message delivery failed: Channel closed");
    }

    #[test]
    fn test_error_debug_formatting() {
        let error = ConnectionError::ClientNotFound("test_client".to_string());
        let debug_string = format!("{:?}", error);
        assert!(debug_string.contains("ClientNotFound"));
        assert!(debug_string.contains("test_client"));
    }

    #[test]
    fn test_server_error_chain() {
        // Test error conversion chain: io::Error -> ServerError
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
        let server_error: ServerError = io_error.into();
        
        match server_error {
            ServerError::BindError(_) => {
                let error_string = format!("{}", server_error);
                assert!(error_string.contains("Failed to bind to address"));
                assert!(error_string.contains("Permission denied"));
            }
            _ => panic!("Expected BindError variant"),
        }
    }

    #[test]
    fn test_connection_error_chain() {
        // Test error conversion chain: ConnectionError -> ServerError
        let connection_error = ConnectionError::InvalidMessage("Bad format".to_string());
        let server_error: ServerError = connection_error.into();
        
        match server_error {
            ServerError::ConnectionError(inner) => {
                match inner {
                    ConnectionError::InvalidMessage(msg) => {
                        assert_eq!(msg, "Bad format");
                    }
                    _ => panic!("Expected InvalidMessage variant"),
                }
            }
            _ => panic!("Expected ConnectionError variant"),
        }
    }

    #[test]
    fn test_error_source_chain() {
        let io_error = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused");
        let server_error = ServerError::BindError(io_error);
        
        // Test that the error source chain is preserved
        assert!(server_error.source().is_some());
        let source = server_error.source().unwrap();
        assert!(source.to_string().contains("Connection refused"));
    }
}