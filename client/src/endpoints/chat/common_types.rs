use serde::{Deserialize, Deserializer, Serialize};

/// Represents the details of a function call in the response.
///
/// # Fields
/// * `name` - The name of the function called
/// * `arguments` - The function arguments as a JSON object
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ChatFunctionCall {
    /// The name of the function being called
    pub name: String,
    /// The arguments to pass to the function, as a JSON object
    #[serde(deserialize_with = "string_or_object_to_value_deserializer")]
    pub arguments: serde_json::Value,
}

pub fn string_or_object_to_value_deserializer<'de, D>(
    deserializer: D,
) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrObject {
        String(String),
        Object(serde_json::Value),
    }

    match StringOrObject::deserialize(deserializer)? {
        StringOrObject::String(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
        StringOrObject::Object(v) => Ok(v),
    }
}

/// Content format that can be either a string or an array of content objects.
///
/// This enum handles the dual content format support:
/// - String format: `"content": "Hello world"`
/// - Array format: `"content": [{"type": "text", "text": "Hello world"}]`
///
/// Note: Null content is represented by wrapping this enum in an `Option`.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum ChatContent {
    /// Simple string content format
    String(String),
    /// Array of structured content objects
    Array(Vec<ContentObject>),
}

/// Represents a single content object.
///
/// This structure supports content represented as an array of typed objects
/// within message content arrays.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ContentObject {
    /// The type of content (typically "text")
    #[serde(rename = "type")]
    pub content_type: String,
    /// The actual text content
    pub text: String,
}

/// Represents a tool call made by the assistant.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ToolCall {
    /// The ID of the tool call
    pub id: String,
    /// The index of the tool call in the list of tool calls
    #[serde(default)]
    pub index: Option<usize>,
    /// The type of the tool (typically "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function call details
    pub function: ChatFunctionCall,
}

/// High-level provider that produced or will consume a given model ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProvider {
    Anthropic,
    OpenAI,
    Zai,
    MoonshotAI,
    Qwen,
    Unknown,
}

impl From<&str> for ModelProvider {
    fn from(value: &str) -> Self {
        // Model IDs are typically in the form "provider/model-name"
        let provider_prefix = value.split('/').next().unwrap_or("").to_lowercase();
        match provider_prefix.as_str() {
            "anthropic" => ModelProvider::Anthropic,
            "openai" => ModelProvider::OpenAI,
            // GLM models
            "z-ai" => ModelProvider::Zai,
            // Kimi models
            "moonshotai" => ModelProvider::MoonshotAI,
            "qwen" => ModelProvider::Qwen,
            _ => ModelProvider::Unknown,
        }
    }
}

impl ModelProvider {
    /// Convenience helper mirroring the original API used in conversions.
    pub fn from_model_id(model_id: &str) -> Self {
        Self::from(model_id)
    }
}

/// Represents a single message in the chat conversation.
///
/// Each message variant has specific content requirements:
/// - System: mandatory content for system-level instructions
/// - User: mandatory content for user input
/// - Assistant: mandatory content for assistant responses (unlike OpenAI where it's optional)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatMessage {
    /// System message with mandatory content
    System {
        /// The message content (string or array of content objects)
        content: ChatContent,
    },
    /// User message with mandatory content
    User {
        /// The message content (string or array of content objects)
        content: ChatContent,
    },
    /// Assistant message with mandatory content
    Assistant {
        /// The message content (string or array of content objects)
        content: ChatContent,
    },
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
        content: ChatContent,
    },
    /// User message with mandatory content
    User {
        /// The message content in either string or array format
        content: ChatContent,
    },
    /// Assistant message with optional content
    Assistant {
        /// The message content in either string or array format, or None for null content
        content: Option<ChatContent>,
        /// Optional tool calls made by assistant messages
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    /// Tool message with mandatory content
    Tool {
        /// The message content in either string or array format
        content: ChatContent,
        /// Tool call ID for tool messages
        tool_call_id: String,
    },
}

impl ChatMessage {
    /// Creates a system message with text content.
    ///
    /// # Arguments
    /// * `text` - The system message text
    ///
    /// # Returns
    /// A new ChatMessage with role "system"
    pub fn system<S: Into<String>>(text: S) -> Self {
        ChatMessage::System {
            content: ChatContent::String(text.into()),
        }
    }

    /// Creates a user message with text content.
    ///
    /// # Arguments
    /// * `text` - The user message text
    ///
    /// # Returns
    /// A new ChatMessage with role "user"
    pub fn user<S: Into<String>>(text: S) -> Self {
        ChatMessage::User {
            content: ChatContent::String(text.into()),
        }
    }

    /// Creates an assistant message with text content.
    ///
    /// # Arguments
    /// * `text` - The assistant message text
    ///
    /// # Returns
    /// A new ChatMessage with role "assistant"
    pub fn assistant<S: Into<String>>(text: S) -> Self {
        ChatMessage::Assistant {
            content: ChatContent::String(text.into()),
        }
    }
}

impl std::fmt::Display for ChatContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text: String = match self {
            ChatContent::String(s) => s.clone(),
            ChatContent::Array(objects) => objects.iter().map(|obj| &obj.text).cloned().collect(),
        };
        write!(f, "{text}")
    }
}

#[cfg(test)]
mod tests {
    use super::ModelProvider;

    #[test]
    fn test_provider_detection_anthropic() {
        assert_eq!(
            ModelProvider::from("anthropic/claude-3-opus"),
            ModelProvider::Anthropic
        );
    }

    #[test]
    fn test_provider_detection_openai() {
        assert_eq!(ModelProvider::from("openai/gpt-4"), ModelProvider::OpenAI);
    }

    #[test]
    fn test_provider_detection_zai() {
        assert_eq!(ModelProvider::from("z-ai/glm-4"), ModelProvider::Zai);
    }

    #[test]
    fn test_provider_detection_moonshotai() {
        assert_eq!(
            ModelProvider::from("moonshotai/moonshot-v1"),
            ModelProvider::MoonshotAI
        );
    }

    #[test]
    fn test_provider_detection_qwen() {
        assert_eq!(ModelProvider::from("qwen/qwen-max"), ModelProvider::Qwen);
    }

    #[test]
    fn test_from_model_id_method() {
        assert_eq!(
            ModelProvider::from_model_id("anthropic/claude-3-haiku"),
            ModelProvider::Anthropic
        );
    }
}
