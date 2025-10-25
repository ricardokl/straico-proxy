// Re-export types from client crate
pub use straico_client::endpoints::chat::{
    ChatChoice, ChatContent, ContentObject, OpenAiChatMessage, OpenAiChatRequest,
    OpenAiChatResponse, OpenAiConversionError, OpenAiFunction, OpenAiNamedToolChoice, OpenAiTool,
    OpenAiToolChoice, StraicoChatResponse, ToolCall, Usage,
};
