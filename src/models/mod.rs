pub mod client;
pub mod config;
pub mod error;
pub mod message;

// Re-export commonly used types
pub use client::{Client, ClientRegistry};
pub use error::{ConnectionError, ServerError};
pub use message::{ClientId, Message, MessageId, MessageType};
