// Re-export types from client crate
pub use straico_client::endpoints::chat::{
    ChatChoice, ChatContent, ContentObject, OpenAiChatMessage, OpenAiChatRequest,
    OpenAiChatResponse, OpenAiFunction, OpenAiNamedToolChoice, OpenAiTool, OpenAiToolChoice,
    StraicoChatResponse, ToolCall, Usage,
};
pub use straico_client::OpenAiConversionError;
