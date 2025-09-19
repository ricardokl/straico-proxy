pub mod chat;
pub mod client;
pub mod endpoints;
pub mod error;

// Re-export commonly used types
pub use endpoints::chat::{ChatRequest, ChatResponse, ChatMessage, ContentObject};
pub use endpoints::completion::{CompletionRequest, Completion};
pub use client::{StraicoClient, StraicoRequestBuilder};
