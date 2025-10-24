use super::{
    ChatMessage, ChatRequest, ChatContent, ContentObject, OpenAiChatMessage,
};
use crate::chat::{Chat, Message, Tool};
use once_cell::sync::Lazy;
use regex::Regex;

static TOOL_CALLS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<tool_calls>(.*)</tool_calls>").unwrap());

impl From<OpenAiChatMessage> for ChatMessage {
    fn from(message: OpenAiChatMessage) -> Self {
        match message {
            OpenAiChatMessage::System { content } => ChatMessage::System { content },
            OpenAiChatMessage::User { content } => ChatMessage::User { content },
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                if let Some(tool_calls) = tool_calls {
                    let tool_calls_str = serde_json::to_string(&tool_calls)
                        .unwrap_or_else(|_| "[]".to_string());
                    let new_content = format!("<tool_calls>{}</tool_calls>", tool_calls_str);
                    ChatMessage::Assistant {
                        content: ChatContent::String(new_content),
                    }
                } else {
                    ChatMessage::Assistant {
                        content: content.unwrap_or(ChatContent::String("".to_string())),
                    }
                }
            }
            OpenAiChatMessage::Tool {
                content,
                tool_call_id,
            } => {
                let tool_output = format!(
                    "<tool_output tool_call_id=\"{}\">{}</tool_output>",
                    tool_call_id,
                    content.to_string()
                );
                ChatMessage::User {
                    content: ChatContent::String(tool_output),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoints::chat::{ChatFunctionCall, ToolCall};

    #[test]
    fn test_openai_to_chat_message_system() {
        let open_ai_msg = OpenAiChatMessage::System {
            content: ChatContent::String("System message".to_string()),
        };
        let chat_msg: ChatMessage = open_ai_msg.into();
        match chat_msg {
            ChatMessage::System { content } => {
                assert_eq!(content.to_string(), "System message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_user() {
        let open_ai_msg = OpenAiChatMessage::User {
            content: ChatContent::String("User message".to_string()),
        };
        let chat_msg: ChatMessage = open_ai_msg.into();
        match chat_msg {
            ChatMessage::User { content } => {
                assert_eq!(content.to_string(), "User message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_assistant_no_tools() {
        let open_ai_msg = OpenAiChatMessage::Assistant {
            content: Some(ChatContent::String("Assistant message".to_string())),
            tool_calls: None,
        };
        let chat_msg: ChatMessage = open_ai_msg.into();
        match chat_msg {
            ChatMessage::Assistant { content } => {
                assert_eq!(content.to_string(), "Assistant message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_assistant_with_tools() {
        let tool_calls = vec![ToolCall {
            id: "tool1".to_string(),
            tool_type: "function".to_string(),
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: "{}".to_string(),
            },
        }];
        let open_ai_msg = OpenAiChatMessage::Assistant {
            content: None,
            tool_calls: Some(tool_calls),
        };
        let chat_msg: ChatMessage = open_ai_msg.into();
        match chat_msg {
            ChatMessage::Assistant { content } => {
                let expected_str =
                    "<tool_calls>[{\"id\":\"tool1\",\"type\":\"function\",\"function\":{\"name\":\"test_func\",\"arguments\":\"{}\"}}]</tool_calls>";
                assert_eq!(content.to_string(), expected_str);
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_openai_to_chat_message_tool() {
        let open_ai_msg = OpenAiChatMessage::Tool {
            content: ChatContent::String("Tool output".to_string()),
            tool_call_id: "tool1".to_string(),
        };
        let chat_msg: ChatMessage = open_ai_msg.into();
        match chat_msg {
            ChatMessage::User { content } => {
                let expected_str =
                    "<tool_output tool_call_id=\"tool1\">Tool output</tool_output>";
                assert_eq!(content.to_string(), expected_str);
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_system() {
        let chat_msg = ChatMessage::System {
            content: ChatContent::String("System message".to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.into();
        match open_ai_msg {
            OpenAiChatMessage::System { content } => {
                assert_eq!(content.to_string(), "System message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_user() {
        let chat_msg = ChatMessage::User {
            content: ChatContent::String("User message".to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.into();
        match open_ai_msg {
            OpenAiChatMessage::User { content } => {
                assert_eq!(content.to_string(), "User message")
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_no_tools() {
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String("Assistant message".to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.into();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert_eq!(
                    content.unwrap().to_string(),
                    "Assistant message"
                );
                assert!(tool_calls.is_none());
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_tools() {
        let tool_calls_str =
            "[{\"id\":\"tool1\",\"type\":\"function\",\"function\":{\"name\":\"test_func\",\"arguments\":\"{}\"}}]";
        let content_str = format!("<tool_calls>{}</tool_calls>", tool_calls_str);
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.into();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_none());
                let expected_tool_calls = vec![ToolCall {
                    id: "tool1".to_string(),
                    tool_type: "function".to_string(),
                    function: ChatFunctionCall {
                        name: "test_func".to_string(),
                        arguments: "{}".to_string(),
                    },
                }];
                assert_eq!(tool_calls.unwrap(), expected_tool_calls);
            }
            _ => panic!("Incorrect message type"),
        }
    }

    #[test]
    fn test_chat_to_openai_message_assistant_with_malformed_tools() {
        let content_str = "<tool_calls>malformed json</tool_calls>";
        let chat_msg = ChatMessage::Assistant {
            content: ChatContent::String(content_str.to_string()),
        };
        let open_ai_msg: OpenAiChatMessage = chat_msg.into();
        match open_ai_msg {
            OpenAiChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_none());
                assert!(tool_calls.unwrap().is_empty());
            }
            _ => panic!("Incorrect message type"),
        }
    }
}

impl From<ChatMessage> for OpenAiChatMessage {
    fn from(message: ChatMessage) -> Self {
        match message {
            ChatMessage::System { content } => OpenAiChatMessage::System { content },
            ChatMessage::User { content } => OpenAiChatMessage::User { content },
            ChatMessage::Assistant { content } => {
                let content_str = content.to_string();
                if let Some(captures) = TOOL_CALLS_REGEX.captures(&content_str) {
                    if let Some(match_str) = captures.get(1) {
                        let tool_calls = serde_json::from_str(match_str.as_str()).unwrap_or(vec![]);
                        return OpenAiChatMessage::Assistant {
                            content: None,
                            tool_calls: Some(tool_calls),
                        };
                    }
                }
                OpenAiChatMessage::Assistant {
                    content: Some(content),
                    tool_calls: None,
                }
            }
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
        let content_vec: Vec<ContentObject> = message.content.map_or(vec![], |c| c.into_content_objects());
        let content = if content_vec.is_empty() {
            ChatContent::String(String::new())
        } else {
            ChatContent::Array(content_vec)
        };
        match message.role.as_str() {
            "system" => ChatMessage::System { content },
            "assistant" => ChatMessage::Assistant { content },
            _ => ChatMessage::User { content },
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
    pub fn build(self) -> ChatRequest<ChatMessage> {
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
        let content = match message {
            ChatMessage::System { content } => content,
            ChatMessage::User { content } => content,
            ChatMessage::Assistant { content } => content,
        };
        match content {
            ChatContent::String(s) => !s.trim().is_empty(),
            ChatContent::Array(objects) => {
                !objects.is_empty() && objects.iter().any(|obj| !obj.text.trim().is_empty())
            }
        }
    }
}
