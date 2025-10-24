// Re-export OpenAI types from client crate
pub use straico_client::endpoints::chat::{
    OpenAiChatMessage, OpenAiChatRequest, OpenAiChatResponse, OpenAiContent, 
    OpenAiContentObject, OpenAiFunctionCall, OpenAiToolCall, OpenAiFunction,
    OpenAiTool, OpenAiToolChoice, OpenAiNamedToolChoice, OpenAiChatChoice,
    OpenAiUsage, OpenAiConversionError,
};

