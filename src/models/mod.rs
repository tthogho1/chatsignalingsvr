pub mod client;
pub mod message;
pub mod error;
pub mod config;

// Re-export commonly used types
pub use client::{Client, ClientRegistry};
pub use message::{ClientId, Message, MessageId, MessageType};
pub use error::{ConnectionError, ServerError};