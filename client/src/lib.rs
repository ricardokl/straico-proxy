pub mod chat;
pub mod client;
pub mod endpoints;
pub mod error;

// Re-export commonly used types
pub use client::{StraicoClient, StraicoRequestBuilder};
pub use endpoints::chat::{
    ChatContent, ChatMessage, ChatRequest, ContentObject,
    OpenAiChatMessage, OpenAiChatRequest, OpenAiChatResponse, OpenAiConversionError, Usage,
};
// ChatResponse is the Straico-specific response, exported from chat module
pub use endpoints::chat::ChatResponse;
