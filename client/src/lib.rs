pub mod client;
pub mod endpoints;
pub mod error;

// Re-export commonly used types
pub use client::{StraicoClient, StraicoRequestBuilder};
pub use endpoints::chat::{
    conversions::OpenAiConversionError, ChatChoice, ChatContent, ChatMessage, ChatRequest,
    ContentObject, MetricBreakdown, OpenAiChatMessage, OpenAiChatRequest, OpenAiChatResponse,
    StraicoChatResponse,
};
