use serde::{Deserialize, Serialize};

/// OpenAI-compatible content format that can be either a string or an array of content objects.
///
/// This enum handles the dual content format support required by the OpenAI API:
/// - String format: `"content": "Hello world"`
/// - Array format: `"content": [{"type": "text", "text": "Hello world"}]`
/// 
/// Note: Null content is represented by wrapping this enum in an `Option`.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum OpenAiContent {
    /// Simple string content format
    String(String),
    /// Array of structured content objects
    Array(Vec<OpenAiContentObject>),
}

/// Represents a single content object in the OpenAI array format.
///
/// This structure matches the OpenAI API specification for content objects
/// within message content arrays.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiContentObject {
    /// The type of content (typically "text")
    #[serde(rename = "type")]
    pub content_type: String,
    /// The actual text content
    pub text: String,
}

/// Represents a function call within a tool call.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiFunctionCall {
    /// The name of the function being called
    pub name: String,
    /// The arguments to pass to the function, as a JSON string
    pub arguments: String,
}

/// Represents a tool call made by the assistant.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiToolCall {
    /// The ID of the tool call
    pub id: String,
    /// The type of the tool (typically "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function call details
    pub function: OpenAiFunctionCall,
}

/// Represents a chat message in OpenAI format.
///
/// This structure is used in both requests and responses.
/// In requests, it represents incoming messages that need to be converted to Straico format.
/// In responses, it appears in the choices array.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum OpenAiChatMessage {
    /// System message with mandatory content
    System {
        /// The message content in either string or array format
        content: OpenAiContent,
    },
    /// User message with mandatory content
    User {
        /// The message content in either string or array format
        content: OpenAiContent,
    },
    /// Assistant message with optional content
    Assistant {
        /// The message content in either string or array format, or None for null content
        content: Option<OpenAiContent>,
        /// Optional tool calls made by assistant messages
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<OpenAiToolCall>>,
    },
    /// Tool message with mandatory content
    Tool {
        /// The message content in either string or array format
        content: OpenAiContent,
        /// Tool call ID for tool messages
        tool_call_id: String,
    },
}
