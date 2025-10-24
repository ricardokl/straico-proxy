// Re-export types from client crate
pub use straico_client::endpoints::chat::{
    ChatContent, ContentObject, OpenAiChatChoice, OpenAiChatMessage,
    OpenAiChatRequest, StraicoChatResponse, OpenAiChatResponse, OpenAiConversionError, OpenAiFunction,
    OpenAiNamedToolChoice, OpenAiTool, ToolCall, OpenAiToolChoice, Usage,
};
