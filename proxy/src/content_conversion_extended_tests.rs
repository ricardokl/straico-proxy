#[cfg(test)]
mod extended_content_conversion_tests {
    use super::*;
    use crate::openai_types::*;
    use crate::content_conversion::*;

    #[test]
    fn test_mixed_content_array() {
        // Test array with multiple text objects of different sizes
        let mixed_content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Short".to_string(),
            },
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "This is a much longer piece of text that should be handled correctly by the conversion system.".to_string(),
            },
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "".to_string(), // Empty text
            },
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Final piece".to_string(),
            },
        ]);

        let converted = convert_openai_content_to_straico(mixed_content);
        
        assert_eq!(converted.len(), 4);
        assert_eq!(converted[0].text, "Short");
        assert!(converted[1].text.len() > 50);
        assert_eq!(converted[2].text, "");
        assert_eq!(converted[3].text, "Final piece");
        
        // All should have text type
        for content_obj in &converted {
            assert_eq!(content_obj.content_type, "text");
        }
    }

    #[test]
    fn test_unicode_content() {
        // Test with emoji and unicode characters
        let unicode_content = OpenAiContent::String(
            "Hello 👋 世界 🌍 Testing unicode: àáâãäå æç èéêë ìíîï ñ òóôõö ùúûü ý 🚀🎉🔥💯".to_string()
        );

        let converted = convert_openai_content_to_straico(unicode_content);
        
        assert_eq!(converted.len(), 1);
        assert!(converted[0].text.contains("👋"));
        assert!(converted[0].text.contains("世界"));
        assert!(converted[0].text.contains("🌍"));
        assert!(converted[0].text.contains("àáâãäå"));
        assert!(converted[0].text.contains("🚀🎉🔥💯"));
    }

    #[test]
    fn test_large_content() {
        // Test with large text content (10KB)
        let large_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(200);
        let large_content = OpenAiContent::String(large_text.clone());

        let converted = convert_openai_content_to_straico(large_content);
        
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].text, large_text);
        assert!(converted[0].text.len() > 10000);
    }

    #[test]
    fn test_edge_cases() {
        // Test empty string content
        let empty_string = OpenAiContent::String("".to_string());
        let converted = convert_openai_content_to_straico(empty_string);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].text, "");

        // Test empty array content
        let empty_array = OpenAiContent::Array(vec![]);
        let converted = convert_openai_content_to_straico(empty_array);
        assert_eq!(converted.len(), 0);

        // Test whitespace-only content
        let whitespace_content = OpenAiContent::String("   \n\t   ".to_string());
        let converted = convert_openai_content_to_straico(whitespace_content);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].text, "   \n\t   ");

        // Test content with special characters
        let special_chars = OpenAiContent::String(r#"Special chars: !@#$%^&*()_+-=[]{}|;':",./<>?"#.to_string());
        let converted = convert_openai_content_to_straico(special_chars);
        assert_eq!(converted.len(), 1);
        assert!(converted[0].text.contains("!@#$%^&*()"));
    }

    #[test]
    fn test_content_validation_edge_cases() {
        // Test validation with edge cases
        
        // Empty string should fail validation
        let empty_string = OpenAiContent::String("".to_string());
        assert!(validate_openai_content(&empty_string).is_err());

        // Whitespace-only string should fail validation
        let whitespace_only = OpenAiContent::String("   \n\t   ".to_string());
        assert!(validate_openai_content(&whitespace_only).is_err());

        // Empty array should fail validation
        let empty_array = OpenAiContent::Array(vec![]);
        assert!(validate_openai_content(&empty_array).is_err());

        // Array with empty text should fail validation
        let array_with_empty = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "".to_string(),
            }
        ]);
        assert!(validate_openai_content(&array_with_empty).is_err());

        // Array with invalid content type should fail validation
        let invalid_type = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "image".to_string(),
                text: "Some text".to_string(),
            }
        ]);
        assert!(validate_openai_content(&invalid_type).is_err());
    }

    #[test]
    fn test_message_validation_comprehensive() {
        // Test comprehensive message validation

        // Valid message
        let valid_message = OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("Valid content".to_string()),
            tool_call_id: None,
            name: None,
        };
        assert!(validate_openai_message(&valid_message).is_ok());

        // Empty role should fail
        let empty_role = OpenAiChatMessage {
            role: "".to_string(),
            content: OpenAiContent::String("Content".to_string()),
            tool_call_id: None,
            name: None,
        };
        assert!(validate_openai_message(&empty_role).is_err());

        // Invalid role should fail
        let invalid_role = OpenAiChatMessage {
            role: "invalid_role".to_string(),
            content: OpenAiContent::String("Content".to_string()),
            tool_call_id: None,
            name: None,
        };
        assert!(validate_openai_message(&invalid_role).is_err());

        // Tool message without tool_call_id should fail
        let tool_without_id = OpenAiChatMessage {
            role: "tool".to_string(),
            content: OpenAiContent::String("Tool response".to_string()),
            tool_call_id: None,
            name: None,
        };
        assert!(validate_openai_message(&tool_without_id).is_err());

        // Tool message with tool_call_id should pass
        let tool_with_id = OpenAiChatMessage {
            role: "tool".to_string(),
            content: OpenAiContent::String("Tool response".to_string()),
            tool_call_id: Some("call_123".to_string()),
            name: None,
        };
        assert!(validate_openai_message(&tool_with_id).is_ok());
    }

    #[test]
    fn test_request_validation_comprehensive() {
        // Test comprehensive request validation

        // Valid request
        let valid_request = OpenAiChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: OpenAiContent::String("Hello".to_string()),
                    tool_call_id: None,
                    name: None,
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            max_completion_tokens: None,
            stream: false,
            tools: None,
        };
        assert!(validate_openai_request(&valid_request).is_ok());

        // Empty model should fail
        let mut empty_model = valid_request.clone();
        empty_model.model = "".to_string();
        assert!(validate_openai_request(&empty_model).is_err());

        // Empty messages should fail
        let mut empty_messages = valid_request.clone();
        empty_messages.messages = vec![];
        assert!(validate_openai_request(&empty_messages).is_err());

        // Invalid temperature should fail
        let mut invalid_temp = valid_request.clone();
        invalid_temp.temperature = Some(-1.0);
        assert!(validate_openai_request(&invalid_temp).is_err());

        let mut invalid_temp_high = valid_request.clone();
        invalid_temp_high.temperature = Some(3.0);
        assert!(validate_openai_request(&invalid_temp_high).is_err());

        // Zero max_tokens should fail
        let mut zero_tokens = valid_request.clone();
        zero_tokens.max_tokens = Some(0);
        assert!(validate_openai_request(&zero_tokens).is_err());

        // Zero max_completion_tokens should fail
        let mut zero_completion_tokens = valid_request.clone();
        zero_completion_tokens.max_completion_tokens = Some(0);
        assert!(validate_openai_request(&zero_completion_tokens).is_err());
    }

    #[test]
    fn test_content_normalization() {
        // Test content normalization to array format

        // String content normalization
        let string_content = OpenAiContent::String("Test content".to_string());
        let normalized = normalize_openai_content_to_array(string_content);
        
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].content_type, "text");
        assert_eq!(normalized[0].text, "Test content");

        // Array content normalization (should remain unchanged)
        let array_content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Array content".to_string(),
            }
        ]);
        let normalized = normalize_openai_content_to_array(array_content);
        
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].content_type, "text");
        assert_eq!(normalized[0].text, "Array content");
    }

    #[test]
    fn test_content_splitting() {
        // Test content splitting functionality
        let long_content = "This is a very long piece of content that should be split into multiple chunks for better processing and handling by the system.";
        
        // Test splitting into small chunks
        let chunks = split_content_into_chunks(long_content, 20);
        assert!(chunks.len() > 1);
        
        // Verify all chunks are within limit
        for chunk in &chunks {
            assert!(chunk.text.len() <= 20);
        }
        
        // Verify content is preserved when reconstructed
        let reconstructed: String = chunks.iter().map(|c| &c.text).cloned().collect();
        assert_eq!(reconstructed, long_content);

        // Test with content shorter than limit
        let short_content = "Short";
        let short_chunks = split_content_into_chunks(short_content, 20);
        assert_eq!(short_chunks.len(), 1);
        assert_eq!(short_chunks[0].text, "Short");
    }

    #[test]
    fn test_content_merging() {
        // Test merging multiple content arrays
        let array1 = vec![
            ContentObject::text("First"),
            ContentObject::text("Second"),
        ];
        
        let array2 = vec![
            ContentObject::text("Third"),
        ];
        
        let array3 = vec![
            ContentObject::text("Fourth"),
            ContentObject::text("Fifth"),
        ];
        
        let merged = merge_content_arrays(vec![array1, array2, array3]);
        
        assert_eq!(merged.len(), 5);
        assert_eq!(merged[0].text, "First");
        assert_eq!(merged[1].text, "Second");
        assert_eq!(merged[2].text, "Third");
        assert_eq!(merged[3].text, "Fourth");
        assert_eq!(merged[4].text, "Fifth");
    }

    #[test]
    fn test_text_extraction() {
        // Test text extraction from content objects
        let content_objects = vec![
            ContentObject::text("Hello "),
            ContentObject::text("world"),
            ContentObject::text("!"),
        ];
        
        let extracted = extract_text_from_content(&content_objects);
        assert_eq!(extracted, "Hello world!");
        
        // Test with empty content
        let empty_content: Vec<ContentObject> = vec![];
        let empty_extracted = extract_text_from_content(&empty_content);
        assert_eq!(empty_extracted, "");
    }

    #[test]
    fn test_system_message_handling() {
        // Test default system message creation
        let default_system = create_default_system_message();
        assert_eq!(default_system.role, "system");
        assert_eq!(default_system.content.len(), 1);
        assert_eq!(default_system.content[0].text, "You are a helpful assistant.");

        // Test ensuring system message exists
        let mut request_without_system = ChatRequest::new()
            .model("test-model")
            .message(ChatMessage::user("Hello"))
            .build();
        
        request_without_system = ensure_system_message(request_without_system);
        assert_eq!(request_without_system.messages.len(), 2);
        assert_eq!(request_without_system.messages[0].role, "system");

        // Test with existing system message
        let mut request_with_system = ChatRequest::new()
            .model("test-model")
            .message(ChatMessage::system("Custom system message"))
            .message(ChatMessage::user("Hello"))
            .build();
        
        let original_len = request_with_system.messages.len();
        request_with_system = ensure_system_message(request_with_system);
        assert_eq!(request_with_system.messages.len(), original_len);
        assert!(request_with_system.messages[0].content[0].text.contains("Custom system"));
    }

    #[test]
    fn test_message_content_validation() {
        // Test message content validation utility
        let valid_message = ChatMessage::new(
            "user",
            vec![ContentObject::text("Valid content")]
        );
        assert!(validate_message_content(&valid_message));

        // Test empty content
        let empty_content_message = ChatMessage::new("user", vec![]);
        assert!(!validate_message_content(&empty_content_message));

        // Test whitespace-only content
        let whitespace_message = ChatMessage::new(
            "user",
            vec![ContentObject::text("   \n\t   ")]
        );
        assert!(!validate_message_content(&whitespace_message));

        // Test mixed valid and invalid content
        let mixed_message = ChatMessage::new(
            "user",
            vec![
                ContentObject::text(""),
                ContentObject::text("Valid content"),
            ]
        );
        assert!(validate_message_content(&mixed_message));
    }
}