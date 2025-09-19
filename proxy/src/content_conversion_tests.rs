#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai_types::{OpenAiContent, OpenAiChatMessage, OpenAiChatRequest, OpenAiContentObject};
    use straico_client::endpoints::chat::{ChatMessage, ContentObject};

    #[test]
    fn test_convert_string_content_to_straico() {
        let openai_content = OpenAiContent::String("Hello, world!".to_string());
        let straico_content = convert_openai_content_to_straico(openai_content);
        
        assert_eq!(straico_content.len(), 1);
        assert_eq!(straico_content[0].content_type, "text");
        assert_eq!(straico_content[0].text, "Hello, world!");
    }

    #[test]
    fn test_convert_array_content_to_straico() {
        let openai_content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Hello, ".to_string(),
            },
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "world!".to_string(),
            },
        ]);
        
        let straico_content = convert_openai_content_to_straico(openai_content);
        
        assert_eq!(straico_content.len(), 2);
        assert_eq!(straico_content[0].content_type, "text");
        assert_eq!(straico_content[0].text, "Hello, ");
        assert_eq!(straico_content[1].content_type, "text");
        assert_eq!(straico_content[1].text, "world!");
    }

    #[test]
    fn test_convert_openai_message_to_straico() {
        let openai_message = OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("Test message".to_string()),
            tool_call_id: None,
            name: None,
        };
        
        let straico_message = convert_openai_message_to_straico(openai_message);
        
        assert_eq!(straico_message.role, "user");
        assert_eq!(straico_message.content.len(), 1);
        assert_eq!(straico_message.content[0].text, "Test message");
    }

    #[test]
    fn test_convert_complete_openai_request() {
        let openai_request = OpenAiChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: OpenAiContent::String("You are a helpful assistant.".to_string()),
                    tool_call_id: None,
                    name: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: OpenAiContent::String("Hello!".to_string()),
                    tool_call_id: None,
                    name: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            max_completion_tokens: None,
            stream: false,
            tools: None,
        };
        
        let result = convert_openai_request_to_straico(openai_request);
        assert!(result.is_ok());
        
        let straico_request = result.unwrap();
        assert_eq!(straico_request.model, "gpt-3.5-turbo");
        assert_eq!(straico_request.messages.len(), 2);
        assert_eq!(straico_request.temperature, Some(0.7));
        assert_eq!(straico_request.max_tokens, Some(100));
    }

    #[test]
    fn test_validate_content_objects_valid() {
        let content = vec![
            ContentObject::text("Hello"),
            ContentObject::text("World"),
        ];
        
        assert!(validate_content_objects(&content).is_ok());
    }

    #[test]
    fn test_validate_content_objects_invalid_type() {
        let content = vec![
            ContentObject::new("image", "data"),
        ];
        
        let result = validate_content_objects(&content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported content type"));
    }

    #[test]
    fn test_validate_content_objects_empty_text() {
        let content = vec![
            ContentObject::text(""),
        ];
        
        let result = validate_content_objects(&content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty text"));
    }

    #[test]
    fn test_validate_openai_content_string() {
        let content = OpenAiContent::String("Valid content".to_string());
        assert!(validate_openai_content(&content).is_ok());
        
        let empty_content = OpenAiContent::String("".to_string());
        assert!(validate_openai_content(&empty_content).is_err());
    }

    #[test]
    fn test_validate_openai_content_array() {
        let valid_content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Valid".to_string(),
            },
        ]);
        assert!(validate_openai_content(&valid_content).is_ok());
        
        let invalid_content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "image".to_string(),
                text: "Invalid".to_string(),
            },
        ]);
        assert!(validate_openai_content(&invalid_content).is_err());
    }

    #[test]
    fn test_normalize_openai_content() {
        let string_content = OpenAiContent::String("Test".to_string());
        let normalized = normalize_openai_content_to_array(string_content);
        
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].content_type, "text");
        assert_eq!(normalized[0].text, "Test");
        
        let array_content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Test".to_string(),
            },
        ]);
        let normalized_array = normalize_openai_content_to_array(array_content);
        assert_eq!(normalized_array.len(), 1);
        assert_eq!(normalized_array[0].text, "Test");
    }

    #[test]
    fn test_split_content_into_chunks() {
        let short_text = "Short";
        let chunks = split_content_into_chunks(short_text, 10);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Short");
        
        let long_text = "This is a very long text that should be split into multiple chunks";
        let chunks = split_content_into_chunks(long_text, 20);
        assert!(chunks.len() > 1);
        
        // Verify all chunks are within the limit
        for chunk in &chunks {
            assert!(chunk.text.len() <= 20);
        }
        
        // Verify content is preserved
        let reconstructed: String = chunks.iter().map(|c| &c.text).cloned().collect();
        assert_eq!(reconstructed, long_text);
    }

    #[test]
    fn test_extract_text_from_content() {
        let content = vec![
            ContentObject::text("Hello, "),
            ContentObject::text("world!"),
        ];
        
        let extracted = extract_text_from_content(&content);
        assert_eq!(extracted, "Hello, world!");
    }

    #[test]
    fn test_ensure_system_message() {
        // Test request without system message
        let mut request = ChatRequest::new()
            .model("test-model")
            .message(ChatMessage::user("Hello"))
            .build();
        
        request = ensure_system_message(request);
        
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, "system");
        
        // Test request with existing system message
        let mut request_with_system = ChatRequest::new()
            .model("test-model")
            .message(ChatMessage::system("Custom system"))
            .message(ChatMessage::user("Hello"))
            .build();
        
        request_with_system = ensure_system_message(request_with_system);
        
        assert_eq!(request_with_system.messages.len(), 2);
        assert_eq!(request_with_system.messages[0].role, "system");
        assert!(request_with_system.messages[0].content[0].text.contains("Custom system"));
    }
}