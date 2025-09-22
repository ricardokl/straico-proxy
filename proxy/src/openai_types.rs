use serde::{Deserialize, Serialize};
use straico_client::endpoints::chat::{ChatMessage, ChatRequest, ContentObject};

/// OpenAI-compatible content format that can be either a string or an array of content objects.
///
/// This enum handles the dual content format support required by the OpenAI API:
/// - String format: `"content": "Hello world"`
/// - Array format: `"content": [{"type": "text", "text": "Hello world"}]`
#[derive(Deserialize, Serialize, Clone, Debug)]
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
#[derive(Deserialize, Serialize, Clone, Debug)]
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
#[derive(Deserialize, Serialize, Clone, Debug)]
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
}

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
            OpenAiContent::Array(objects) => {
                objects.iter().map(|obj| {
                    ContentObject::new(&obj.content_type, &obj.text)
                }).collect()
            }
        }
    }

    /// Checks if the content is empty.
    ///
    /// # Returns
    /// True if the content is empty or contains only empty text
    pub fn is_empty(&self) -> bool {
        match self {
            OpenAiContent::String(text) => text.trim().is_empty(),
            OpenAiContent::Array(objects) => {
                objects.is_empty() || objects.iter().all(|obj| obj.text.trim().is_empty())
            }
        }
    }

    /// Gets the text content as a single string.
    ///
    /// For array format, concatenates all text objects.
    ///
    /// # Returns
    /// String representation of the content
    pub fn to_string(&self) -> String {
        match self {
            OpenAiContent::String(text) => text.clone(),
            OpenAiContent::Array(objects) => {
                objects.iter()
                    .map(|obj| &obj.text)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("")
            }
        }
    }

    /// Validates that the content is well-formed.
    ///
    /// # Returns
    /// Ok(()) if valid, Err(String) with error message if invalid
    pub fn validate(&self) -> Result<(), String> {
        match self {
            OpenAiContent::String(text) => {
                if text.trim().is_empty() {
                    Err("String content cannot be empty".to_string())
                } else {
                    Ok(())
                }
            }
            OpenAiContent::Array(objects) => {
                if objects.is_empty() {
                    return Err("Content array cannot be empty".to_string());
                }
                
                for (i, obj) in objects.iter().enumerate() {
                    if obj.content_type.trim().is_empty() {
                        return Err(format!("Content object {} has empty type", i));
                    }
                    if obj.text.trim().is_empty() {
                        return Err(format!("Content object {} has empty text", i));
                    }
                    // Currently only support "text" type
                    if obj.content_type != "text" {
                        return Err(format!("Unsupported content type: {}", obj.content_type));
                    }
                }
                Ok(())
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

    /// Validates that the message is well-formed.
    ///
    /// # Returns
    /// Ok(()) if valid, Err(String) with error message if invalid
    pub fn validate(&self) -> Result<(), String> {
        // Validate role
        if self.role.trim().is_empty() {
            return Err("Message role cannot be empty".to_string());
        }

        // Validate supported roles
        match self.role.as_str() {
            "system" | "user" | "assistant" | "tool" => {},
            _ => return Err(format!("Unsupported message role: {}", self.role)),
        }

        // Validate content
        self.content.validate()?;

        // Tool messages should have tool_call_id
        if self.role == "tool" && self.tool_call_id.is_none() {
            return Err("Tool messages must have tool_call_id".to_string());
        }

        Ok(())
    }
}

impl OpenAiChatRequest {
    /// Converts OpenAI chat request to Straico ChatRequest format.
    ///
    /// # Returns
    /// A ChatRequest with converted message format
    pub fn to_straico_request(&self) -> Result<ChatRequest, String> {
        // Validate the request first
        self.validate()?;

        let messages: Vec<ChatMessage> = self.messages.iter()
            .map(|msg| msg.to_straico_message())
            .collect();

        let mut builder = ChatRequest::builder()
            .model(&self.model)
            .messages(messages);

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

    /// Validates that the request is well-formed.
    ///
    /// # Returns
    /// Ok(()) if valid, Err(String) with error message if invalid
    pub fn validate(&self) -> Result<(), String> {
        // Validate model
        if self.model.trim().is_empty() {
            return Err("Model cannot be empty".to_string());
        }

        // Validate messages
        if self.messages.is_empty() {
            return Err("Messages array cannot be empty".to_string());
        }

        for (i, message) in self.messages.iter().enumerate() {
            message.validate()
                .map_err(|e| format!("Message {}: {}", i, e))?;
        }

        // Validate temperature range
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err("Temperature must be between 0.0 and 2.0".to_string());
            }
        }

        // Validate max_tokens
        if let Some(tokens) = self.max_tokens {
            if tokens == 0 {
                return Err("max_tokens must be greater than 0".to_string());
            }
        }

        if let Some(tokens) = self.max_completion_tokens {
            if tokens == 0 {
                return Err("max_completion_tokens must be greater than 0".to_string());
            }
        }

        Ok(())
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

/// Utility functions for content format conversion.
pub mod conversion_utils {
    use super::*;

    /// Converts a string to OpenAI content format.
    pub fn string_to_openai_content(text: String) -> OpenAiContent {
        OpenAiContent::String(text)
    }

    /// Converts an array of content objects to OpenAI content format.
    pub fn array_to_openai_content(objects: Vec<OpenAiContentObject>) -> OpenAiContent {
        OpenAiContent::Array(objects)
    }

    /// Normalizes OpenAI content to always be in array format.
    pub fn normalize_to_array(content: OpenAiContent) -> Vec<OpenAiContentObject> {
        match content {
            OpenAiContent::String(text) => {
                vec![OpenAiContentObject {
                    content_type: "text".to_string(),
                    text,
                }]
            }
            OpenAiContent::Array(objects) => objects,
        }
    }

    /// Validates an entire OpenAI chat request.
    pub fn validate_openai_request(request: &OpenAiChatRequest) -> Result<(), String> {
        request.validate()
    }

    /// Converts OpenAI request to Straico format with validation.
    pub fn convert_openai_to_straico(request: OpenAiChatRequest) -> Result<ChatRequest, String> {
        request.to_straico_request()
    }
}