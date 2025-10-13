use std::env;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use crate::models::error::ServerError;

/// Server configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_address: IpAddr,
    pub port: u16,
    pub max_connections: usize,
    pub log_level: String,
}

impl ServerConfig {
    /// Load configuration from environment variables with fallback to defaults
    pub fn from_env() -> Result<Self, ServerError> {
        // Load environment variables from .env file if present
        dotenv::dotenv().ok();

        let bind_address = Self::parse_bind_address()?;
        let port = Self::parse_port()?;
        let max_connections = Self::parse_max_connections()?;
        let log_level = Self::parse_log_level();

        Ok(ServerConfig {
            bind_address,
            port,
            max_connections,
            log_level,
        })
    }

    /// Get the socket address for binding
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind_address, self.port)
    }

    fn parse_bind_address() -> Result<IpAddr, ServerError> {
        let addr_str = env::var("SERVER_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
        
        IpAddr::from_str(&addr_str).map_err(|e| {
            ServerError::ConfigError(format!("Invalid bind address '{}': {}", addr_str, e))
        })
    }

    fn parse_port() -> Result<u16, ServerError> {
        let port_str = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
        
        port_str.parse::<u16>().map_err(|e| {
            ServerError::ConfigError(format!("Invalid port '{}': {}", port_str, e))
        }).and_then(|port| {
            if port == 0 {
                Err(ServerError::ConfigError("Port cannot be 0".to_string()))
            } else {
                Ok(port)
            }
        })
    }

    fn parse_max_connections() -> Result<usize, ServerError> {
        let max_conn_str = env::var("MAX_CONNECTIONS").unwrap_or_else(|_| "1000".to_string());
        
        max_conn_str.parse::<usize>().map_err(|e| {
            ServerError::ConfigError(format!("Invalid max_connections '{}': {}", max_conn_str, e))
        }).and_then(|max_conn| {
            if max_conn == 0 {
                Err(ServerError::ConfigError("max_connections cannot be 0".to_string()))
            } else {
                Ok(max_conn)
            }
        })
    }

    fn parse_log_level() -> String {
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        
        // Validate log level
        match log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => log_level.to_lowercase(),
            _ => {
                eprintln!("Warning: Invalid log level '{}', using 'info' as default", log_level);
                "info".to_string()
            }
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_address: IpAddr::from_str("127.0.0.1").unwrap(),
            port: 8080,
            max_connections: 1000,
            log_level: "info".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use serial_test::serial;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, IpAddr::from_str("127.0.0.1").unwrap());
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_socket_addr() {
        let config = ServerConfig::default();
        let addr = config.socket_addr();
        assert_eq!(addr.ip(), IpAddr::from_str("127.0.0.1").unwrap());
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    #[serial]
    fn test_config_from_env_with_defaults() {
        // Clear ALL environment variables to test defaults
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");

        let config = ServerConfig::from_env().unwrap();
        assert_eq!(config.bind_address, IpAddr::from_str("127.0.0.1").unwrap());
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    #[serial]
    fn test_config_from_env_with_custom_values() {
        // Clear ALL environment variables first, then set custom values
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
        
        env::set_var("SERVER_BIND_ADDRESS", "0.0.0.0");
        env::set_var("SERVER_PORT", "9090");
        env::set_var("MAX_CONNECTIONS", "500");
        env::set_var("LOG_LEVEL", "debug");

        let config = ServerConfig::from_env().unwrap();
        assert_eq!(config.bind_address, IpAddr::from_str("0.0.0.0").unwrap());
        assert_eq!(config.port, 9090);
        assert_eq!(config.max_connections, 500);
        assert_eq!(config.log_level, "debug");

        // Clean up
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
    }

    #[test]
    #[serial]
    fn test_invalid_bind_address() {
        // Clear ALL environment variables first, then set test values
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
        
        env::set_var("SERVER_PORT", "8080");
        env::set_var("SERVER_BIND_ADDRESS", "invalid-address");
        
        let result = ServerConfig::from_env();
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Invalid bind address"), 
                "Expected 'Invalid bind address' in error message, got: {}", error_message);

        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_invalid_port() {
        // Clear ALL environment variables first, then set test values
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
        
        env::set_var("SERVER_PORT", "invalid-port");
        
        let result = ServerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid port"));

        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_zero_port() {
        // Clear ALL environment variables first, then set test values
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
        
        env::set_var("SERVER_PORT", "0");
        
        let result = ServerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Port cannot be 0"));

        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_invalid_max_connections() {
        // Clear ALL environment variables first, then set test values
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
        
        env::set_var("SERVER_PORT", "8080");
        env::set_var("MAX_CONNECTIONS", "invalid-number");
        
        let result = ServerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid max_connections"));

        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_zero_max_connections() {
        // Clear ALL environment variables first, then set test values
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
        
        env::set_var("SERVER_PORT", "8080");
        env::set_var("MAX_CONNECTIONS", "0");
        
        let result = ServerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_connections cannot be 0"));

        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_invalid_log_level_uses_default() {
        // Clear ALL environment variables first, then set test values
        env::remove_var("SERVER_BIND_ADDRESS");
        env::remove_var("SERVER_PORT");
        env::remove_var("MAX_CONNECTIONS");
        env::remove_var("LOG_LEVEL");
        
        env::set_var("SERVER_PORT", "8080");
        env::set_var("LOG_LEVEL", "invalid-level");
        
        let config = ServerConfig::from_env().unwrap();
        assert_eq!(config.log_level, "info");

        env::remove_var("LOG_LEVEL");
        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_valid_log_levels() {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        
        for level in &valid_levels {
            // Clear ALL environment variables first, then set test values
            env::remove_var("SERVER_BIND_ADDRESS");
            env::remove_var("SERVER_PORT");
            env::remove_var("MAX_CONNECTIONS");
            env::remove_var("LOG_LEVEL");
            
            env::set_var("SERVER_PORT", "8080");
            env::set_var("LOG_LEVEL", level);
            let config = ServerConfig::from_env().unwrap();
            assert_eq!(config.log_level, *level);
        }

        // Final cleanup
        env::remove_var("SERVER_PORT");
        env::remove_var("LOG_LEVEL");
    }
}