use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common_types::OpenAiChatMessage;

/// Represents the details of a function call in the response.
///
/// # Fields
/// * `name` - The name of the function called
/// * `arguments` - The function arguments as a JSON string
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ChatFunctionCall {
    /// The name of the function being called
    pub name: String,
    /// The arguments to pass to the function, as a JSON string
    pub arguments: String,
}

/// OpenAI chat completion response.
///
/// This structure represents the response returned to OpenAI-compatible clients.
/// It follows the OpenAI API specification for chat completion responses.
///
/// # Fields
/// * `id` - Unique identifier for the completion
/// * `object` - The type of object (typically "chat.completion")
/// * `created` - Unix timestamp of when the completion was created
/// * `model` - The model used for the completion
/// * `choices` - Array of completion choices
/// * `usage` - Token usage statistics
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OpenAiChatResponse {
    /// Unique identifier for the completion
    pub id: String,
    /// The type of object (typically "chat.completion")
    pub object: String,
    /// Unix timestamp of when the completion was created
    pub created: u64,
    /// The model used for the completion
    pub model: String,
    /// Array of completion choices
    pub choices: Vec<OpenAiChatChoice>,
    /// Token usage statistics
    pub usage: Usage,
}

/// Represents a single choice in the OpenAI chat completion response.
///
/// Each choice contains a message and metadata about the completion.
///
/// # Fields
/// * `index` - Zero-based position of this choice in the list
/// * `message` - The generated message
/// * `finish_reason` - Why the model stopped generating
/// * `logprobs` - Optional log probabilities for the tokens
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OpenAiChatChoice {
    /// Zero-based position of this choice in the list
    pub index: u8,
    /// The generated message
    pub message: OpenAiChatMessage,
    /// Why the model stopped generating (e.g., "stop", "length", "tool_calls")
    pub finish_reason: String,
    /// Optional log probabilities for the tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<String>,
}

/// Token usage statistics for the chat completion.
///
/// This structure tracks token consumption for the request and response.
///
/// # Fields
/// * `prompt_tokens` - Number of tokens in the prompt/input
/// * `completion_tokens` - Number of tokens in the completion/output
/// * `total_tokens` - Total combined token count
/// * `completion_tokens_details` - Additional details about completion tokens
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Usage {
    /// Number of tokens in the prompt/input
    pub prompt_tokens: u32,
    /// Number of tokens in the completion/output
    pub completion_tokens: u32,
    /// Total combined token count
    pub total_tokens: u32,
    /// Additional details about completion tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<Value>,
}

/// Breakdown of metrics (price or word count) for input, output, and total.
///
/// Used for both price (as floats) and word counts (deserialized as floats but
/// representing integers). Using f64 allows handling both use cases.
///
/// # Fields
/// * `input` - Metric for the input/prompt
/// * `output` - Metric for the generated output/completion
/// * `total` - Total combined metric
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MetricBreakdown {
    /// Metric for the input/prompt
    pub input: f64,
    /// Metric for the generated output/completion
    pub output: f64,
    /// Total combined metric
    pub total: f64,
}
