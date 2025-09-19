use serde::{Deserialize, Serialize};

/// Response structure for the new Straico chat endpoint.
///
/// This struct represents the response from the `/v0/chat/completions` endpoint.
/// The structure may need refinement based on actual API responses.
///
/// # Fields
/// * `choices` - Array of generated response choices
/// * `model` - The model that generated the response
/// * `usage` - Optional token usage statistics
/// * `id` - Unique identifier for this completion
/// * `object` - The type of object (e.g., "chat.completion")
/// * `created` - Unix timestamp of when this completion was created
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatResponse {
    /// Array of generated response choices
    pub choices: Vec<ChatChoice>,
    /// The model that generated the response
    pub model: String,
    /// Optional token usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ChatUsage>,
    /// Unique identifier for this completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The type of object (e.g., "chat.completion")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// Unix timestamp of when this completion was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
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
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatChoice {
    /// The generated response message
    pub message: ChatResponseMessage,
    /// Reason why the model stopped generating
    pub finish_reason: String,
    /// Zero-based position of this choice in the list of responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u8>,
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
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatResponseMessage {
    /// The role of the message sender
    pub role: String,
    /// The message content
    pub content: Option<ChatResponseContent>,
    /// Optional tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
}

/// Represents the content of a chat response message.
///
/// This enum handles different content formats that the new chat endpoint
/// might return - either simple text or structured content arrays.
///
/// # Variants
/// * `Text` - Simple text content
/// * `Array` - Array of structured content objects
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ChatResponseContent {
    /// Simple text content
    Text(String),
    /// Array of structured content objects
    Array(Vec<ChatContentObject>),
}

/// Represents a structured content object in the response.
///
/// Similar to the request content objects, but for responses.
///
/// # Fields
/// * `content_type` - The type of content (typically "text")
/// * `text` - The actual text content
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatContentObject {
    /// The type of content object
    #[serde(rename = "type")]
    pub content_type: String,
    /// The text content
    pub text: String,
}

/// Represents a tool call in the chat response.
///
/// This structure is compatible with the existing tool call format
/// but adapted for the new chat endpoint.
///
/// # Fields
/// * `id` - Unique identifier for the tool call
/// * `function` - The function call details
/// * `tool_type` - The type of tool (typically "function")
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatToolCall {
    /// Unique identifier for the tool call
    pub id: String,
    /// The function call details
    pub function: ChatFunctionCall,
    /// The type of tool (typically "function")
    #[serde(rename = "type")]
    pub tool_type: String,
}

/// Represents the details of a function call in the response.
///
/// # Fields
/// * `name` - The name of the function called
/// * `arguments` - The function arguments as a JSON string
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatFunctionCall {
    /// The name of the function called
    pub name: String,
    /// The function arguments as a JSON string
    pub arguments: String,
}

/// Token usage statistics for the chat completion.
///
/// This structure tracks token consumption for billing and monitoring purposes.
///
/// # Fields
/// * `prompt_tokens` - Number of tokens in the input/prompt
/// * `completion_tokens` - Number of tokens in the generated completion
/// * `total_tokens` - Total combined token count
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatUsage {
    /// Number of tokens in the input/prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the generated completion
    pub completion_tokens: u32,
    /// Total combined token count
    pub total_tokens: u32,
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
            choice.message.tool_calls.as_ref()
                .map_or(false, |calls| !calls.is_empty())
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
        self.message.content.as_ref().map(|content| content.to_string())
    }
}

impl ChatResponseContent {
    /// Converts the content to a string representation.
    ///
    /// # Returns
    /// String representation of the content
    pub fn to_string(&self) -> String {
        match self {
            ChatResponseContent::Text(text) => text.clone(),
            ChatResponseContent::Array(objects) => {
                objects.iter()
                    .map(|obj| &obj.text)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("")
            }
        }
    }

    /// Checks if the content is empty.
    ///
    /// # Returns
    /// True if the content is empty or contains only empty text
    pub fn is_empty(&self) -> bool {
        match self {
            ChatResponseContent::Text(text) => text.is_empty(),
            ChatResponseContent::Array(objects) => {
                objects.is_empty() || objects.iter().all(|obj| obj.text.is_empty())
            }
        }
    }
}

impl std::fmt::Display for ChatResponseContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<String> for ChatResponseContent {
    fn from(text: String) -> Self {
        ChatResponseContent::Text(text)
    }
}

impl From<Vec<ChatContentObject>> for ChatResponseContent {
    fn from(objects: Vec<ChatContentObject>) -> Self {
        ChatResponseContent::Array(objects)
    }
}