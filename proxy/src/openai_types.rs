use serde::{Deserialize, Serialize};
use straico_client::endpoints::chat::{ChatMessage, ChatRequest, ContentObject};

/// OpenAI-compatible content format that can be either a string or an array of content objects.
///
/// This enum handles the dual content format support required by the OpenAI API:
/// - String format: `"content": "Hello world"`
/// - Array format: `"content": [{"type": "text", "text": "Hello world"}]`
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

/// Represents a chat message in OpenAI format.
///
/// This structure handles incoming OpenAI-style messages that need to be
/// converted to the new Straico chat format.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OpenAiChatMessage {
    /// The role of the message sender (system, user, assistant, tool)
    pub role: String,
    /// The message content in either string or array format
    pub content: OpenAiContent,
    /// Optional tool call ID for tool messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Optional name for function/tool messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Represents a complete OpenAI chat request.
///
/// This structure handles incoming OpenAI-compatible requests that need to be
/// converted to the new Straico chat format.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAiChatRequest {
    /// The model identifier to use for completion
    pub model: String,
    /// Array of chat messages in OpenAI format
    pub messages: Vec<OpenAiChatMessage>,
    /// Optional temperature parameter (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Optional maximum number of completion tokens (alias for max_tokens)
    #[serde(alias = "max_completion_tokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Optional tools/functions available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>, // Will be handled in Phase 2
    /// Optional tool choice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

use std::fmt;

impl OpenAiContent {
    /// Converts OpenAI content format to Straico ContentObject vector.
    ///
    /// # Returns
    /// A vector of ContentObject representing the same content in Straico format
    pub fn to_straico_content(&self) -> Vec<ContentObject> {
        match self {
            OpenAiContent::String(text) => {
                vec![ContentObject::text(text.clone())]
            }
            OpenAiContent::Array(objects) => objects
                .iter()
                .map(|obj| ContentObject::new(&obj.content_type, &obj.text))
                .collect(),
        }
    }
}

impl fmt::Display for OpenAiContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenAiContent::String(text) => write!(f, "{}", text),
            OpenAiContent::Array(objects) => {
                let text: String = objects.iter().map(|obj| &obj.text).cloned().collect();
                write!(f, "{}", text)
            }
        }
    }
}

impl OpenAiChatMessage {
    /// Converts OpenAI chat message to Straico ChatMessage format.
    ///
    /// # Returns
    /// A ChatMessage with converted content format
    pub fn to_straico_message(&self) -> ChatMessage {
        ChatMessage::new(&self.role, self.content.to_straico_content())
    }
}

impl OpenAiChatRequest {
    /// Converts OpenAI chat request to Straico ChatRequest format.
    ///
    /// # Returns
    /// A ChatRequest with converted message format
    pub fn to_straico_request(&self) -> Result<ChatRequest, String> {
        let messages: Vec<ChatMessage> = self
            .messages
            .iter()
            .map(|msg| msg.to_straico_message())
            .collect();

        let mut builder = ChatRequest::builder().model(&self.model).messages(messages);

        // Handle max_tokens vs max_completion_tokens
        let max_tokens = self.max_tokens.or(self.max_completion_tokens);
        if let Some(tokens) = max_tokens {
            builder = builder.max_tokens(tokens);
        }

        if let Some(temp) = self.temperature {
            builder = builder.temperature(temp);
        }

        Ok(builder.build())
    }
}

impl From<OpenAiContentObject> for ContentObject {
    fn from(obj: OpenAiContentObject) -> Self {
        ContentObject::new(obj.content_type, obj.text)
    }
}

impl From<OpenAiChatMessage> for ChatMessage {
    fn from(msg: OpenAiChatMessage) -> Self {
        msg.to_straico_message()
    }
}
