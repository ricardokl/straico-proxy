use super::{ChatContentObject, ChatMessage, ChatRequest, ChatResponseContent, ContentObject};
use crate::chat::{Chat, Tool};
use crate::endpoints::completion::completion_response::{Content, Message, TextObject};

/// Conversion utilities for the new chat endpoint format.
///
/// This module provides conversion functions between different content formats
/// and compatibility with existing OpenAI-style messages.
///
/// Note: Direct From implementations for Vec<ContentObject> violate orphan rules
/// Instead, we provide utility functions for these conversions
impl From<Content> for Vec<ContentObject> {
    /// Converts existing Content enum to new ContentObject format.
    ///
    /// # Arguments
    /// * `content` - The existing Content enum
    ///
    /// # Returns
    /// A vector of ContentObject representing the same content
    fn from(content: Content) -> Self {
        match content {
            Content::Text(text) => vec![ContentObject::text(text.as_ref())],
            Content::TextArray(text_objects) => text_objects
                .into_iter()
                .map(|obj| match obj {
                    TextObject::Text { text } => ContentObject::text(text.as_ref()),
                })
                .collect(),
        }
    }
}

impl From<Message> for ChatMessage {
    /// Converts existing Message enum to new ChatMessage format.
    ///
    /// # Arguments
    /// * `message` - The existing Message enum
    ///
    /// # Returns
    /// A ChatMessage with the same role and converted content
    fn from(message: Message) -> Self {
        match message {
            Message::User { content } => ChatMessage::new("user", content.into()),
            Message::Assistant {
                content,
                tool_calls: _,
            } => {
                // Note: Tool calls are handled separately in the new format
                match content {
                    Some(content) => ChatMessage::new("assistant", content.into()),
                    None => ChatMessage::new("assistant", vec![]),
                }
            }
            Message::System { content } => ChatMessage::new("system", content.into()),
            Message::Tool { content } => ChatMessage::new("tool", content.into()),
        }
    }
}

impl From<Chat> for Vec<ChatMessage> {
    /// Converts existing Chat to new ChatMessage vector format.
    ///
    /// # Arguments
    /// * `chat` - The existing Chat wrapper
    ///
    /// # Returns
    /// A vector of ChatMessage representing the same conversation
    fn from(chat: Chat) -> Self {
        chat.iter().map(|message| message.clone().into()).collect()
    }
}

impl From<Vec<ChatContentObject>> for Content {
    /// Converts new ChatContentObject vector to existing Content format.
    ///
    /// # Arguments
    /// * `objects` - Vector of ChatContentObject
    ///
    /// # Returns
    /// Content enum representing the same content
    fn from(objects: Vec<ChatContentObject>) -> Self {
        if objects.len() == 1 {
            Content::Text(objects[0].text.clone().into())
        } else {
            let text_objects = objects
                .into_iter()
                .map(|obj| TextObject::Text {
                    text: obj.text.into(),
                })
                .collect();
            Content::TextArray(text_objects)
        }
    }
}

impl From<ChatResponseContent> for Content {
    /// Converts ChatResponseContent to existing Content format.
    ///
    /// # Arguments
    /// * `content` - The ChatResponseContent to convert
    ///
    /// # Returns
    /// Content enum representing the same content
    fn from(content: ChatResponseContent) -> Self {
        match content {
            ChatResponseContent::Text(text) => Content::Text(text.into()),
            ChatResponseContent::Array(objects) => objects.into(),
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
        let chat_messages: Vec<ChatMessage> = chat.into();
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
