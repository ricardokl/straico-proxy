use serde::{Deserialize, Serialize};

use super::{
    common_types::ToolCall,
    ChatContent, MetricBreakdown, Usage,
};

/// Response structure for the Straico chat endpoint.
///
/// This struct represents the response from the `/v0/chat/completions` endpoint.
///
/// # Fields
/// * `id` - Unique identifier for this completion
/// * `model` - The model that generated the response
/// * `object` - The type of object (e.g., "chat.completion")
/// * `created` - Unix timestamp of when this completion was created
/// * `choices` - Array of generated response choices
/// * `usage` - Token usage statistics
/// * `price` - Price breakdown for the completion
/// * `words` - Word count breakdown
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatResponse {
    /// Unique identifier for this completion
    pub id: String,
    /// The model that generated the response
    pub model: String,
    /// The type of object (e.g., "chat.completion")
    pub object: String,
    /// Unix timestamp of when this completion was created
    pub created: u64,
    /// Array of generated response choices
    pub choices: Vec<ChatChoice>,
    /// Token usage statistics
    pub usage: Usage,
    /// Price breakdown for the completion
    pub price: MetricBreakdown,
    /// Word count breakdown
    pub words: MetricBreakdown,
}

/// Represents a single choice/response from the chat completion.
///
/// Each choice contains the generated message and metadata about why
/// the generation stopped.
///
/// # Fields
/// * `message` - The generated response message
/// * `finish_reason` - Why the model stopped generating (e.g., "stop", "length", "tool_calls")
/// * `index` - Zero-based position of this choice in the list of responses
///
/// # Note
/// Optional fields not implemented: "logprobs", "native_finish_reason"
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatChoice {
    /// The generated response message
    pub message: Message,
    /// Reason why the model stopped generating
    pub finish_reason: String,
    /// Zero-based position of this choice in the list of responses
    pub index: u8,
}

/// Represents a message in the chat response.
///
/// This structure contains the role and content of the generated message.
/// For the new chat endpoint, content may be structured differently than
/// the current completion endpoint.
///
/// # Fields
/// * `role` - The role of the message sender (typically "assistant")
/// * `content` - The message content (may be string or structured)
/// * `tool_calls` - Optional tool calls made by the assistant
///
/// # Note
/// Optional fields not implemented: "refusal", "reasoning"
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

impl ChatResponse {
    /// Gets the first choice from the response.
    ///
    /// # Returns
    /// An Option containing the first ChatChoice, or None if no choices exist
    pub fn first_choice(&self) -> Option<&ChatChoice> {
        self.choices.first()
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
        self.choices.iter().any(|choice| {
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
