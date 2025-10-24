pub mod chat;
pub mod client;
pub mod endpoints;
pub mod error;

// Re-export commonly used types
pub use client::{StraicoClient, StraicoRequestBuilder};
pub use endpoints::chat::{
    ChatChoice, ChatContent, ChatMessage, ChatRequest, ContentObject, Message, MetricBreakdown,
    OpenAiChatChoice, OpenAiChatMessage, OpenAiChatRequest, OpenAiChatResponse,
    OpenAiConversionError, StraicoChatResponse,
};
