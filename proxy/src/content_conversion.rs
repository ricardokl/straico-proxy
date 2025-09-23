use crate::openai_types::{
    OpenAiChatMessage, OpenAiChatRequest, OpenAiContent, OpenAiContentObject,
};
use straico_client::endpoints::chat::{
    ChatMessage, ChatRequest, ChatResponseContent, ChatResponseMessage, ChatToolCall,
    ContentObject,
};

/// Content conversion utilities for transforming OpenAI format to Straico format.
///
/// This module provides comprehensive conversion functions to handle the dual content
/// format support required by the OpenAI API compatibility layer.
///

/// Converts OpenAI content format to Straico ContentObject vector.
///
/// Handles both string and array content formats from OpenAI requests.
///
/// # Arguments
/// * `content` - The OpenAI content in either string or array format
///
/// # Returns
/// A vector of ContentObject in Straico format
pub fn convert_openai_content_to_straico(content: OpenAiContent) -> Vec<ContentObject> {
    content.to_straico_content()
}

/// Converts OpenAI chat message to Straico ChatMessage format.
///
/// # Arguments
/// * `message` - The OpenAI chat message to convert
///
/// # Returns
/// A ChatMessage in Straico format
pub fn convert_openai_message_to_straico(message: OpenAiChatMessage) -> ChatMessage {
    message.to_straico_message()
}

/// Converts complete OpenAI chat request to Straico ChatRequest format.
///
/// This is the main conversion function that handles the complete request transformation.
///
/// # Arguments
/// * `openai_request` - The OpenAI chat request to convert
///
/// # Returns
/// Result containing ChatRequest in Straico format or error message
pub fn convert_openai_request_to_straico(
    openai_request: OpenAiChatRequest,
) -> Result<ChatRequest, String> {
    openai_request.to_straico_request()
}

/// Validates that content objects are well-formed and supported.
///
/// # Arguments
/// * `content` - The content objects to validate
///
/// # Returns
/// Ok(()) if valid, Err(String) with error message if invalid
pub fn validate_content_objects(content: &[ContentObject]) -> Result<(), String> {
    if content.is_empty() {
        return Err("Content array cannot be empty".to_string());
    }

    for (i, obj) in content.iter().enumerate() {
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

/// Normalizes OpenAI content to always be in array format.
///
/// This is useful for consistent processing regardless of input format.
///
/// # Arguments
/// * `content` - The OpenAI content to normalize
///
/// # Returns
/// Vector of OpenAiContentObject representing the content
pub fn normalize_openai_content_to_array(content: OpenAiContent) -> Vec<OpenAiContentObject> {
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

/// Converts Straico ContentObject back to OpenAI format.
///
/// This is useful for response conversion or testing.
///
/// # Arguments
/// * `content` - Vector of Straico ContentObject
///
/// # Returns
/// OpenAiContent in array format
pub fn convert_straico_content_to_openai(content: Vec<ContentObject>) -> OpenAiContent {
    let objects = content
        .into_iter()
        .map(|obj| OpenAiContentObject {
            content_type: obj.content_type,
            text: obj.text,
        })
        .collect();
    OpenAiContent::Array(objects)
}

/// Merges multiple content arrays into a single array.
///
/// # Arguments
/// * `content_arrays` - Vector of content arrays to merge
///
/// # Returns
/// Single merged vector of ContentObject
pub fn merge_content_arrays(content_arrays: Vec<Vec<ContentObject>>) -> Vec<ContentObject> {
    content_arrays.into_iter().flatten().collect()
}

/// Splits large content into smaller chunks for processing.
///
/// # Arguments
/// * `content` - The content to split
/// * `max_length` - Maximum length per chunk
///
/// # Returns
/// Vector of content chunks
pub fn split_content_into_chunks(content: &str, max_length: usize) -> Vec<ContentObject> {
    if content.len() <= max_length {
        return vec![ContentObject::text(content)];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < content.len() {
        let end = std::cmp::min(start + max_length, content.len());
        let chunk = &content[start..end];
        chunks.push(ContentObject::text(chunk));
        start = end;
    }

    chunks
}

/// Extracts text content from any content format.
///
/// # Arguments
/// * `content` - The content to extract text from
///
/// # Returns
/// Concatenated text string
pub fn extract_text_from_content(content: &[ContentObject]) -> String {
    content
        .iter()
        .map(|obj| &obj.text)
        .cloned()
        .collect::<Vec<_>>()
        .join("")
}

/// Creates a default system message for chat requests.
///
/// # Returns
/// ChatMessage with default system instructions
pub fn create_default_system_message() -> ChatMessage {
    ChatMessage::system("You are a helpful assistant.")
}

/// Ensures a chat request has a system message.
///
/// If no system message exists, adds a default one at the beginning.
///
/// # Arguments
/// * `request` - The chat request to check and modify
///
/// # Returns
/// Modified chat request with guaranteed system message
pub fn ensure_system_message(mut request: ChatRequest) -> ChatRequest {
    // Check if first message is a system message
    if request.messages.is_empty() || request.messages[0].role != "system" {
        // Insert default system message at the beginning
        request.messages.insert(0, create_default_system_message());
    }
    request
}

/// Parses tool calls from a chat message and populates the tool_calls field.
///
/// # Arguments
/// * `message` - The chat message to parse
///
/// # Returns
/// Modified chat message with tool_calls populated if any were found
pub fn parse_tool_calls(mut message: ChatResponseMessage) -> ChatResponseMessage {
    if message.role != "assistant" {
        return message;
    }

    if let Some(content) = &message.content {
        let mut tool_calls = Vec::new();
        let mut text_content = String::new();
        let mut tool_code_found = false;

        match content {
            ChatResponseContent::Array(content_objects) => {
                for content_obj in content_objects {
                    if content_obj.content_type == "tool_code" {
                        tool_code_found = true;
                        if let Ok(parsed_tool_calls) =
                            serde_json::from_str::<Vec<ChatToolCall>>(&content_obj.text)
                        {
                            tool_calls.extend(parsed_tool_calls);
                        }
                    } else {
                        text_content.push_str(&content_obj.text);
                    }
                }
            }
            ChatResponseContent::Text(text) => {
                text_content.push_str(text);
            }
        }

        if tool_code_found {
            if text_content.is_empty() {
                message.content = None;
            } else {
                message.content = Some(ChatResponseContent::Text(text_content));
            }

            if !tool_calls.is_empty() {
                if let Some(existing_tool_calls) = &mut message.tool_calls {
                    existing_tool_calls.extend(tool_calls);
                } else {
                    message.tool_calls = Some(tool_calls);
                }
            }
        }
    }

    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use straico_client::endpoints::chat::{
        ChatContentObject, ChatFunctionCall, ChatResponseContent, ChatResponseMessage,
        ChatToolCall,
    };

    #[test]
    fn test_parse_tool_calls_single_tool_call() {
        let tool_call = ChatToolCall {
            id: "1".to_string(),
            function: ChatFunctionCall {
                name: "test_function".to_string(),
                arguments: "{}".to_string(),
            },
            tool_type: "function".to_string(),
        };
        let message = ChatResponseMessage {
            role: "assistant".to_string(),
            content: Some(ChatResponseContent::Array(vec![ChatContentObject {
                content_type: "tool_code".to_string(),
                text: serde_json::to_string(&vec![tool_call.clone()]).unwrap(),
            }])),
            tool_calls: None,
        };

        let parsed_message = parse_tool_calls(message);
        assert!(parsed_message.content.is_none());
        assert!(parsed_message.tool_calls.is_some());
        let tool_calls = parsed_message.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "1");
    }

    #[test]
    fn test_parse_tool_calls_multiple_tool_calls() {
        let tool_call1 = ChatToolCall {
            id: "1".to_string(),
            function: ChatFunctionCall {
                name: "test_function1".to_string(),
                arguments: "{}".to_string(),
            },
            tool_type: "function".to_string(),
        };
        let tool_call2 = ChatToolCall {
            id: "2".to_string(),
            function: ChatFunctionCall {
                name: "test_function2".to_string(),
                arguments: "{}".to_string(),
            },
            tool_type: "function".to_string(),
        };
        let message = ChatResponseMessage {
            role: "assistant".to_string(),
            content: Some(ChatResponseContent::Array(vec![ChatContentObject {
                content_type: "tool_code".to_string(),
                text: serde_json::to_string(&vec![tool_call1.clone(), tool_call2.clone()])
                    .unwrap(),
            }])),
            tool_calls: None,
        };

        let parsed_message = parse_tool_calls(message);
        assert!(parsed_message.content.is_none());
        assert!(parsed_message.tool_calls.is_some());
        let tool_calls = parsed_message.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0].id, "1");
        assert_eq!(tool_calls[1].id, "2");
    }

    #[test]
    fn test_parse_tool_calls_mixed_content() {
        let tool_call = ChatToolCall {
            id: "1".to_string(),
            function: ChatFunctionCall {
                name: "test_function".to_string(),
                arguments: "{}".to_string(),
            },
            tool_type: "function".to_string(),
        };
        let message = ChatResponseMessage {
            role: "assistant".to_string(),
            content: Some(ChatResponseContent::Array(vec![
                ChatContentObject {
                    content_type: "text".to_string(),
                    text: "Hello ".to_string(),
                },
                ChatContentObject {
                    content_type: "tool_code".to_string(),
                    text: serde_json::to_string(&vec![tool_call.clone()]).unwrap(),
                },
                ChatContentObject {
                    content_type: "text".to_string(),
                    text: "world!".to_string(),
                },
            ])),
            tool_calls: None,
        };

        let parsed_message = parse_tool_calls(message);
        assert!(parsed_message.content.is_some());
        let content = parsed_message.content.unwrap().as_string();
        assert_eq!(content, "Hello world!");
        assert!(parsed_message.tool_calls.is_some());
        let tool_calls = parsed_message.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "1");
    }

    #[test]
    fn test_parse_tool_calls_invalid_json() {
        let message = ChatResponseMessage {
            role: "assistant".to_string(),
            content: Some(ChatResponseContent::Array(vec![ChatContentObject {
                content_type: "tool_code".to_string(),
                text: "[{]".to_string(),
            }])),
            tool_calls: None,
        };

        let parsed_message = parse_tool_calls(message);
        assert!(parsed_message.content.is_none());
        assert!(parsed_message.tool_calls.is_none());
    }

    #[test]
    fn test_parse_tool_calls_no_tool_code() {
        let message = ChatResponseMessage {
            role: "assistant".to_string(),
            content: Some(ChatResponseContent::Array(vec![ChatContentObject {
                content_type: "text".to_string(),
                text: "Hello world".to_string(),
            }])),
            tool_calls: None,
        };

        let parsed_message = parse_tool_calls(message.clone());
        assert!(parsed_message.content.is_some());
        let content = parsed_message.content.unwrap().as_string();
        assert_eq!(content, "Hello world");
        assert!(parsed_message.tool_calls.is_none());
    }

    #[test]
    fn test_parse_tool_calls_non_assistant_role() {
        let message = ChatResponseMessage {
            role: "user".to_string(),
            content: Some(ChatResponseContent::Array(vec![ChatContentObject {
                content_type: "tool_code".to_string(),
                text: "[]".to_string(),
            }])),
            tool_calls: None,
        };

        let parsed_message = parse_tool_calls(message.clone());
        assert!(parsed_message.content.is_some());
        assert!(parsed_message.tool_calls.is_none());
    }

    #[test]
    fn test_convert_string_content() {
        let content = OpenAiContent::String("Hello world".to_string());
        let result = convert_openai_content_to_straico(content);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content_type, "text");
        assert_eq!(result[0].text, "Hello world");
    }

    #[test]
    fn test_convert_array_content() {
        let content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Hello".to_string(),
            },
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: " world".to_string(),
            },
        ]);

        let result = convert_openai_content_to_straico(content);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Hello");
        assert_eq!(result[1].text, " world");
    }

    #[test]
    fn test_validate_content_objects() {
        let valid_content = vec![ContentObject::text("Hello"), ContentObject::text("World")];

        assert!(validate_content_objects(&valid_content).is_ok());

        let invalid_content = vec![ContentObject::new("image", "data")];

        assert!(validate_content_objects(&invalid_content).is_err());
    }

    #[test]
    fn test_normalize_content() {
        let string_content = OpenAiContent::String("Test".to_string());
        let normalized = normalize_openai_content_to_array(string_content);

        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].content_type, "text");
        assert_eq!(normalized[0].text, "Test");
    }

    #[test]
    fn test_split_content_into_chunks() {
        let long_text = "a".repeat(150);
        let chunks = split_content_into_chunks(&long_text, 50);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text.len(), 50);
        assert_eq!(chunks[1].text.len(), 50);
        assert_eq!(chunks[2].text.len(), 50);
    }
}

