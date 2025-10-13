# Structured Logging Implementation

This document describes the structured logging implementation for the WebSocket Chat and Signaling Server.

## Overview

The server uses the `tracing` crate for structured logging with JSON output format. This provides machine-readable logs that can be easily parsed by log aggregation systems like ELK stack, Splunk, or cloud logging services.

## Configuration

Logging is configured through environment variables:

- `LOG_LEVEL`: Sets the logging level (trace, debug, info, warn, error)
- `RUST_LOG`: Optional override for fine-grained logging control

### Log Levels

- **trace**: Most verbose, includes function entry/exit and detailed execution flow
- **debug**: Detailed information useful for debugging, includes connection details and message routing
- **info**: General operational information (default level)
- **warn**: Warning messages about potential issues
- **error**: Only error conditions

## Structured Fields

The logging implementation includes structured fields for easy filtering and analysis:

### Server Operations

- `bind_address`: Server bind address
- `port`: Server port number
- `max_connections`: Maximum connection limit
- `log_level`: Current log level

### Client Management

- `client_id`: Unique client identifier
- `peer_address`: Client's network address
- `current_connections`: Current number of connected clients
- `total_connections`: Total connections after operation
- `connected_at`: Client connection timestamp
- `connection_duration_seconds`: How long client was connected

### Message Routing

- `message_id`: Unique message identifier
- `sender_id`: Message sender's client ID
- `target_user_id`: Target recipient's client ID
- `message_type`: Type of message (text_chat, webrtc_signaling, generic_message)
- `content_length`: Length of message content
- `signaling_type`: Type of WebRTC signaling (offer, answer, ice-candidate)

### Broadcast Operations

- `recipient_count`: Number of recipients for broadcast
- `successful_deliveries`: Number of successful message deliveries
- `failed_deliveries`: Number of failed message deliveries

### Error Context

- `error`: Error message or description
- `message_preview`: Preview of problematic message content

## Log Examples

### Client Connection

```json
{
  "timestamp": "2023-10-13T10:30:00.123Z",
  "level": "INFO",
  "fields": {
    "client_id": "550e8400-e29b-41d4-a716-446655440000",
    "peer_address": "192.168.1.100:54321",
    "total_connections": 5
  },
  "target": "websocket_chat_signaling_server::lib",
  "message": "Client connected successfully"
}
```

### Message Routing

```json
{
  "timestamp": "2023-10-13T10:30:15.456Z",
  "level": "INFO",
  "fields": {
    "client_id": "550e8400-e29b-41d4-a716-446655440000",
    "message_id": "msg-123456",
    "target_user_id": "550e8400-e29b-41d4-a716-446655440001",
    "content_length": 25,
    "message_type": "text_chat"
  },
  "target": "websocket_chat_signaling_server::lib",
  "message": "Routing text chat message"
}
```

### Error Logging

```json
{
  "timestamp": "2023-10-13T10:30:30.789Z",
  "level": "ERROR",
  "fields": {
    "client_id": "550e8400-e29b-41d4-a716-446655440000",
    "error": "Failed to parse JSON message: expected `,` or `}` at line 1 column 15",
    "message_preview": "{\"invalid\": json"
  },
  "target": "websocket_chat_signaling_server::lib",
  "message": "Message parsing failed"
}
```

## Instrumentation

The logging implementation uses `#[instrument]` attributes on key functions to automatically add context and trace function execution:

- Server startup and shutdown
- Client connection handling
- Message processing and routing
- Error handling

## Performance Considerations

- Structured logging adds minimal overhead
- JSON serialization is optimized for performance
- Log levels can be adjusted to reduce verbosity in production
- Async logging prevents blocking the main application threads

## Integration with Monitoring Systems

The structured JSON format is compatible with:

- **ELK Stack**: Elasticsearch, Logstash, and Kibana
- **Splunk**: Direct JSON ingestion
- **Cloud Logging**: AWS CloudWatch, Google Cloud Logging, Azure Monitor
- **Prometheus**: Can be used with log-based metrics
- **Grafana**: Log visualization and alerting

## Usage Examples

### Running with Debug Logging

```bash
LOG_LEVEL=debug cargo run
```

### Running the Logging Demo

```bash
cargo run --example logging_demo
```

### Production Configuration

```bash
LOG_LEVEL=info
RUST_LOG=websocket_chat_signaling_server=info
```

## Requirements Compliance

This implementation satisfies the following requirements:

- **1.3**: Logs client connections and disconnections with timestamps
- **1.4**: Logs connection cleanup and resource management
- **2.2**: Logs message routing errors with context
- **4.4**: Logs WebRTC signaling errors and delivery failures

The structured logging provides comprehensive visibility into server operations while maintaining high performance and compatibility with modern logging infrastructure.
