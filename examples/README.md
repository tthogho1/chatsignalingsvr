# WebSocket Client Examples

This directory contains example implementations demonstrating how to interact with the WebSocket Chat and Signaling Server.

## WebSocket Client Example

The `websocket_client.rs` example provides a comprehensive WebSocket client that can connect to the server and demonstrate all supported message types.

### Usage

```bash
# Run the example client in interactive mode (default)
cargo run --example websocket_client

# Connect to a specific server
cargo run --example websocket_client -- --server ws://127.0.0.1:8080

# Run in demo mode (automatically sends example messages)
cargo run --example websocket_client -- --mode demo

# Run in test mode (validates server functionality)
cargo run --example websocket_client -- --mode test

# Use a custom client ID
cargo run --example websocket_client -- --client-id my_client_123
```

### Modes

#### Interactive Mode

In interactive mode, you can type commands to send different types of messages:

- `/direct <target_id> <message>` - Send a direct message to a specific client
- `/broadcast <message>` - Send a broadcast message to all clients
- `/webrtc <target_id> <type>` - Send WebRTC signaling data (offer, answer, ice-candidate)
- `/generic <target_id> <content>` - Send a generic message
- `/help` - Show available commands
- `/quit` - Exit the client

Example session:

```
💬 > /broadcast Hello everyone!
📢 Sent broadcast message: Hello everyone!

💬 > /direct client2 How are you?
📤 Sent direct message to client2: How are you?

💬 > /webrtc peer1 offer
🎥 Sent WebRTC offer to peer1

💬 > /quit
```

#### Demo Mode

Demo mode automatically sends a sequence of example messages to demonstrate all server functionality:

1. Broadcast message
2. Direct message
3. WebRTC offer
4. WebRTC answer
5. ICE candidate
6. Generic message

#### Test Mode

Test mode sends various messages and validates that the server accepts them correctly. It's useful for automated testing of server functionality.

### Message Types Demonstrated

#### Text Chat Messages

```json
{
  "id": "uuid-here",
  "sender_id": "client_id",
  "timestamp": "2023-01-01T00:00:00Z",
  "message_type": {
    "TextChat": {
      "target_user_id": "target_client_id", // or null for broadcast
      "content": "Hello, World!"
    }
  }
}
```

#### WebRTC Signaling Messages

```json
{
  "id": "uuid-here",
  "sender_id": "client_id",
  "timestamp": "2023-01-01T00:00:00Z",
  "message_type": {
    "WebRTCSignaling": {
      "target_user_id": "peer_client_id",
      "signaling_data": {
        "type": "offer",
        "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\n..."
      }
    }
  }
}
```

#### Generic Messages

```json
{
  "id": "uuid-here",
  "sender_id": "client_id",
  "timestamp": "2023-01-01T00:00:00Z",
  "message_type": {
    "GenericMessage": {
      "target_user_id": "target_client_id",
      "content": "custom_command_data"
    }
  }
}
```

## Testing with Multiple Clients

To test the server with multiple clients, open multiple terminals and run:

```bash
# Terminal 1
cargo run --example websocket_client -- --client-id client1

# Terminal 2
cargo run --example websocket_client -- --client-id client2

# Terminal 3
cargo run --example websocket_client -- --client-id client3
```

Then you can send messages between the clients:

In client1:

```
💬 > /direct client2 Hello from client1!
💬 > /broadcast Hello everyone from client1!
```

In client2:

```
💬 > /direct client1 Hi back from client2!
💬 > /webrtc client3 offer
```

## Running the Server

Before running the client examples, make sure the server is running:

```bash
# Start the server (in another terminal)
cargo run

# Or with custom configuration
cargo run -- --port 8080 --bind-address 0.0.0.0 --max-connections 100
```

## Other Examples

- `logging_demo.rs` - Demonstrates the logging system
- `cli_demo.rs` - Shows command-line interface usage

## Dependencies

The WebSocket client example uses:

- `tokio-tungstenite` for WebSocket communication
- `futures-util` for async stream handling
- `serde_json` for JSON serialization
- `clap` for command-line argument parsing

All dependencies are already included in the main `Cargo.toml` file.
