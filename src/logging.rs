use tracing::{Level, info};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::config::ServerConfig;

/// Initialize structured logging based on server configuration
pub fn init_logging(config: &ServerConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse log level from config
    let log_level = parse_log_level(&config.log_level)?;
    
    // Create environment filter with the configured log level
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // If no RUST_LOG env var is set, use the configured log level
            EnvFilter::new(format!("{}={}", env!("CARGO_PKG_NAME").replace('-', "_"), log_level))
        });

    // Initialize tracing subscriber with structured JSON logging
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::CLOSE)
                .json()
        )
        .try_init()?;

    info!(
        log_level = %config.log_level,
        bind_address = %config.bind_address,
        port = config.port,
        max_connections = config.max_connections,
        "Logging initialized successfully"
    );

    Ok(())
}

/// Parse log level string to tracing Level
pub fn parse_log_level(level_str: &str) -> Result<Level, Box<dyn std::error::Error + Send + Sync>> {
    match level_str.to_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        _ => Err(format!("Invalid log level: {}", level_str).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn test_parse_log_level() {
        assert!(matches!(parse_log_level("trace").unwrap(), Level::TRACE));
        assert!(matches!(parse_log_level("debug").unwrap(), Level::DEBUG));
        assert!(matches!(parse_log_level("info").unwrap(), Level::INFO));
        assert!(matches!(parse_log_level("warn").unwrap(), Level::WARN));
        assert!(matches!(parse_log_level("error").unwrap(), Level::ERROR));
        
        // Test case insensitive
        assert!(matches!(parse_log_level("INFO").unwrap(), Level::INFO));
        assert!(matches!(parse_log_level("Debug").unwrap(), Level::DEBUG));
        
        // Test invalid level
        assert!(parse_log_level("invalid").is_err());
    }

    #[test]
    fn test_init_logging_with_valid_config() {
        let config = ServerConfig {
            bind_address: IpAddr::from_str("127.0.0.1").unwrap(),
            port: 8080,
            max_connections: 1000,
            log_level: "info".to_string(),
        };

        // Note: We can't easily test the actual initialization in unit tests
        // because tracing can only be initialized once per process
        // This test mainly verifies the function signature and basic validation
        let level = parse_log_level(&config.log_level);
        assert!(level.is_ok());
    }
}