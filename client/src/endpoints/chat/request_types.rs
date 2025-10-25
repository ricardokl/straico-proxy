use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common_types::ChatMessage;

/// A request structure for the Straico chat endpoint.
///
/// This struct represents a request to the `/v0/chat/completions` endpoint with support
/// for the new message format that uses content arrays instead of formatted prompts.
///
/// # Fields
/// * `model` - Single model identifier (unlike completion endpoint which supports multiple)
/// * `messages` - Array of chat messages with structured content
/// * `temperature` - Optional parameter controlling randomness in generation (0.0 to 2.0)
/// * `max_tokens` - Optional maximum number of tokens to generate
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ChatRequest<T> {
    /// The language model to use for generating the chat completion
    pub model: String,
    /// Array of messages forming the conversation context
    pub messages: Vec<T>,
    /// Optional parameter controlling randomness in generation (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Represents a complete OpenAI chat request.
///
/// This structure handles incoming OpenAI-compatible requests that need to be
/// converted to the new Straico chat format.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAiChatRequest<T> {
    #[serde(flatten)]
    pub chat_request: ChatRequest<T>,
    /// Optional maximum number of completion tokens (alias for max_tokens)
    #[serde(alias = "max_completion_tokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Optional tools/functions available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAiTool>>,
    /// Optional tool choice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<OpenAiToolChoice>,
}

/// Represents a function definition within a tool.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiFunction {
    /// The name of the function
    pub name: String,
    /// A description of what the function does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The parameters the function accepts, described as a JSON Schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// Represents a tool available to the model.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "function")]
pub enum OpenAiTool {
    Function(OpenAiFunction),
}

/// Represents a tool choice option.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum OpenAiToolChoice {
    /// A string value like "none", "auto", or "required"
    String(String),
    /// An object specifying a specific tool to use
    Object(OpenAiTool),
}

impl ChatRequest<ChatMessage> {
    /// Creates a new ChatRequest builder.
    ///
    /// # Returns
    /// A new `ChatRequestBuilder` instance for constructing the request
    pub fn builder() -> super::ChatRequestBuilder {
        super::ChatRequestBuilder::default()
    }
}
