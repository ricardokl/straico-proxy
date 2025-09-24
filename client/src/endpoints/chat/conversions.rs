use super::{
    ChatMessage, ChatRequest, ChatResponseContent, ContentObject,
    chat_request::ContentObject as RequestContentObject,
    chat_response::ChatContentObject as ResponseContentObject,
};
use crate::chat::{Chat, Message, Tool};

impl From<RequestContentObject> for ResponseContentObject {
    fn from(obj: RequestContentObject) -> Self {
        ResponseContentObject {
            content_type: obj.content_type,
            text: obj.text,
        }
    }
}

impl From<ResponseContentObject> for RequestContentObject {
    fn from(obj: ResponseContentObject) -> Self {
        RequestContentObject {
            content_type: obj.content_type,
            text: obj.text,
        }
    }
}

/// Conversion utilities for the new chat endpoint format.
///
/// This module provides conversion functions between different content formats
/// and compatibility with existing OpenAI-style messages.
///
/// Note: Direct From implementations for Vec<ContentObject> violate orphan rules
/// Instead, we provide utility functions for these conversions
impl From<Message> for ChatMessage {
    fn from(message: Message) -> Self {
        let content = message.content.map_or(vec![], |c| c.into());
        ChatMessage::new(message.role, content)
    }
}

impl From<ChatResponseContent> for Vec<RequestContentObject> {
    fn from(content: ChatResponseContent) -> Self {
        match content {
            ChatResponseContent::Text(text) => vec![RequestContentObject::text(text)],
            ChatResponseContent::Array(objects) => objects.into_iter().map(|o| o.into()).collect(),
        }
    }
}

/// Builder for creating ChatRequest instances with OpenAI compatibility.
pub struct OpenAiChatRequestBuilder {
    model: Option<String>,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tools: Option<Vec<Tool>>,
}

impl OpenAiChatRequestBuilder {
    /// Creates a new OpenAI-compatible chat request builder.
    pub fn new() -> Self {
        Self {
            model: None,
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            tools: None,
        }
    }

    /// Sets the model for the request.
    pub fn model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Adds messages from an existing Chat.
    pub fn messages_from_chat(mut self, chat: Chat) -> Self {
        let chat_messages: Vec<ChatMessage> = chat.0.into_iter().map(|m| m.into()).collect();
        self.messages.extend(chat_messages);
        self
    }

    /// Adds a single message.
    pub fn message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Sets the temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the max tokens.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Sets the tools (for future tool support).
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Builds the ChatRequest.
    ///
    /// Note: Tools are not yet embedded in the new format.
    /// This will be implemented in Phase 2.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model.expect("Model must be set"),
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

impl Default for OpenAiChatRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for content conversion.
pub mod utils {
    use super::*;

    /// Converts any string-like content to ContentObject vector.
    pub fn to_content_objects<S: Into<String>>(text: S) -> Vec<ContentObject> {
        vec![ContentObject::text(text)]
    }

    /// Converts a string to ContentObject vector.
    pub fn string_to_content_objects(text: String) -> Vec<ContentObject> {
        vec![ContentObject::text(text)]
    }

    /// Converts a string slice to ContentObject vector.
    pub fn str_to_content_objects(text: &str) -> Vec<ContentObject> {
        vec![ContentObject::text(text)]
    }

    /// Merges multiple ContentObject vectors.
    pub fn merge_content_objects(mut vectors: Vec<Vec<ContentObject>>) -> Vec<ContentObject> {
        if vectors.is_empty() {
            return Vec::new();
        }

        let mut result = vectors.remove(0);
        for mut vector in vectors {
            result.append(&mut vector);
        }
        result
    }

    /// Converts ContentObject vector to a single text string.
    pub fn content_objects_to_string(objects: &[ContentObject]) -> String {
        objects
            .iter()
            .map(|obj| &obj.text)
            .cloned()
            .collect::<Vec<_>>()
            .join("")
    }

    /// Creates a system message with default assistant instructions.
    pub fn default_system_message() -> ChatMessage {
        ChatMessage::system("You are a helpful assistant.")
    }

    /// Validates that a ChatMessage has valid content.
    pub fn validate_message_content(message: &ChatMessage) -> bool {
        !message.content.is_empty()
            && message
                .content
                .iter()
                .any(|obj| !obj.text.trim().is_empty())
    }
}
