use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common_types::{OpenAiChatMessage, ToolCall};
use crate::endpoints::chat::ChatContent;

/// Generic chat completion response structure.
///
/// This structure can be used for both OpenAI-compatible and Straico-specific
/// responses by parameterizing the type of `choices`.
///
/// # Type Parameters
/// * `T` - The type of the items in the `choices` vector.
///
/// # Fields
/// * `id` - Unique identifier for the completion
/// * `object` - The type of object (typically "chat.completion")
/// * `created` - Unix timestamp of when the completion was created
/// * `model` - The model used for the completion
/// * `choices` - Array of completion choices
/// * `usage` - Token usage statistics
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatResponse<T> {
    /// Unique identifier for the completion
    pub id: String,
    /// The type of object (typically "chat.completion")
    pub object: String,
    /// Unix timestamp of when the completion was created
    pub created: u64,
    /// The model used for the completion
    pub model: String,
    /// Array of completion choices
    pub choices: Vec<T>,
    /// Token usage statistics
    pub usage: Usage,
}

/// Straico-specific chat completion response.
///
/// This structure extends the generic `ChatResponse` with additional fields
/// specific to the Straico API, such as `price` and `words` breakdowns.
///
/// # Fields
/// This struct flattens all fields from `ChatResponse<ChatChoice>` and adds:
/// * `price` - Price breakdown for the completion
/// * `words` - Word count breakdown
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StraicoChatResponse {
    /// Flattened fields from the generic ChatResponse
    #[serde(flatten)]
    pub response: ChatResponse<ChatChoice>,
    /// Price breakdown for the completion
    pub price: MetricBreakdown,
    /// Word count breakdown
    pub words: MetricBreakdown,
}

/// Type alias for an OpenAI-compatible chat completion response.
///
/// This uses the generic `ChatResponse` with `OpenAiChatChoice` as the choice type.
pub type OpenAiChatResponse = ChatResponse<OpenAiChatChoice>;

/// Represents a single choice in the Straico chat completion response.
///
/// # Fields
/// * `message` - The generated response message
/// * `finish_reason` - Why the model stopped generating (e.g., "stop", "length", "tool_calls")
/// * `index` - Zero-based position of this choice in the list of responses
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatChoice {
    /// The generated response message
    pub message: Message,
    /// Reason why the model stopped generating
    pub finish_reason: String,
    /// Zero-based position of this choice in the list of responses
    pub index: u8,
}

/// Represents a message in the Straico chat response.
///
/// # Fields
/// * `role` - The role of the message sender (typically "assistant")
/// * `content` - The message content (may be string or structured)
/// * `tool_calls` - Optional tool calls made by the assistant
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
    /// The role of the message sender
    pub role: String,
    /// The message content
    pub content: Option<ChatContent>,
    /// Optional tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Represents a single choice in the OpenAI chat completion response.
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
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
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

impl From<Message> for OpenAiChatMessage {
    fn from(value: Message) -> Self {
        OpenAiChatMessage::Assistant {
            content: value.content,
            tool_calls: value.tool_calls,
        }
    }
}

impl StraicoChatResponse {
    /// Gets the first choice from the response.
    ///
    /// # Returns
    /// An Option containing the first ChatChoice, or None if no choices exist
    pub fn first_choice(&self) -> Option<&ChatChoice> {
        self.response.choices.first()
    }

    /// Gets the content of the first choice as a string.
    ///
    /// # Returns
    /// An Option containing the content string, or None if no content exists
    pub fn first_content(&self) -> Option<String> {
        self.first_choice()
            .and_then(|choice| choice.message.content.as_ref())
            .map(|content| content.to_string())
    }

    /// Checks if the response contains tool calls.
    ///
    /// # Returns
    /// True if any choice contains tool calls, false otherwise
    pub fn has_tool_calls(&self) -> bool {
        self.response.choices.iter().any(|choice| {
            choice
                .message
                .tool_calls
                .as_ref()
                .is_some_and(|calls| !calls.is_empty())
        })
    }
}

impl ChatChoice {
    /// Checks if this choice finished due to tool calls.
    ///
    /// # Returns
    /// True if the finish reason is "tool_calls"
    pub fn finished_with_tool_calls(&self) -> bool {
        self.finish_reason == "tool_calls"
    }

    /// Gets the content as a string if available.
    ///
    /// # Returns
    /// An Option containing the content string, or None if no content exists
    pub fn content_string(&self) -> Option<String> {
        self.message
            .content
            .as_ref()
            .map(|content| content.to_string())
    }
}
