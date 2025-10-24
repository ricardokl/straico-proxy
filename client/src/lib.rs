pub mod chat;
pub mod client;
pub mod endpoints;
pub mod error;

// Re-export commonly used types
pub use client::{StraicoClient, StraicoRequestBuilder};
pub use endpoints::chat::{
    ChatMessage, ChatRequest, ChatResponse, ContentObject,
    OpenAiChatMessage, OpenAiChatRequest, OpenAiChatResponse,
    OpenAiContent, OpenAiContentObject, OpenAiConversionError,
};
