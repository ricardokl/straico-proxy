pub mod client;
pub mod endpoints;
pub mod error;

// Re-export commonly used types
pub use client::{StraicoClient, StraicoRequestBuilder};
pub use endpoints::chat::{
    ChatChoice, ChatContent, ChatMessage, ChatRequest, ContentObject, MetricBreakdown,
    OpenAiChatMessage, OpenAiChatRequest, OpenAiChatResponse, OpenAiConversionError,
    StraicoChatResponse,
};
