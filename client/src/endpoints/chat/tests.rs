#[cfg(test)]
mod tests {

    use crate::endpoints::chat::{
        ChatChoice, ChatMessage, ChatRequest, ChatResponse, ChatResponseContent, ChatUsage,
        ContentObject, Message,
    };

    #[test]
    fn test_chat_message_convenience_methods() {
        let system_msg = ChatMessage::system("System prompt");
        match system_msg {
            ChatMessage::System { content } => {
                assert_eq!(content[0].text, "System prompt");
            }
            _ => panic!("Expected System variant"),
        }

        let user_msg = ChatMessage::user("User input");
        match user_msg {
            ChatMessage::User { content } => {
                assert_eq!(content[0].text, "User input");
            }
            _ => panic!("Expected User variant"),
        }

        let assistant_msg = ChatMessage::assistant("Assistant response");
        match assistant_msg {
            ChatMessage::Assistant { content } => {
                assert_eq!(content[0].text, "Assistant response");
            }
            _ => panic!("Expected Assistant variant"),
        }
    }

    #[test]
    fn test_chat_message_serialization() {
        let system_msg = ChatMessage::system("System prompt");
        let json = serde_json::to_string(&system_msg).unwrap();
        println!("System JSON: {}", json);
        assert!(json.contains(r#""role":"system""#));
        assert!(json.contains(r#""content":"#));

        let user_msg = ChatMessage::user("User input");
        let json = serde_json::to_string(&user_msg).unwrap();
        println!("User JSON: {}", json);
        assert!(json.contains(r#""role":"user""#));

        let assistant_msg = ChatMessage::assistant("Assistant response");
        let json = serde_json::to_string(&assistant_msg).unwrap();
        println!("Assistant JSON: {}", json);
        assert!(json.contains(r#""role":"assistant""#));
        
        // Verify the structure is correct - role is at the root level
        let system_value: serde_json::Value = serde_json::from_str(&serde_json::to_string(&system_msg).unwrap()).unwrap();
        assert_eq!(system_value["role"], "system");
        assert!(system_value["content"].is_array());
    }

    #[test]
    fn test_content_object_creation() {
        let text_content = ContentObject::text("Hello world");
        assert_eq!(text_content.content_type, "text");
        assert_eq!(text_content.text, "Hello world");

        let custom_content = ContentObject::new("custom", "Custom content");
        assert_eq!(custom_content.content_type, "custom");
        assert_eq!(custom_content.text, "Custom content");
    }

    #[test]
    fn test_chat_request_builder_pattern() {
        let request = ChatRequest::builder()
            .model("test-model")
            .message(ChatMessage::system("System"))
            .message(ChatMessage::user("User"))
            .temperature(0.5)
            .max_tokens(100)
            .build();

        assert_eq!(request.model, "test-model");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.max_tokens, Some(100));
    }

    #[test]
    fn test_chat_response_methods() {
        let response = create_test_chat_response();

        assert!(response.first_choice().is_some());
        assert_eq!(
            response.first_content(),
            Some("Test response content".to_string())
        );
        assert!(!response.has_tool_calls());
    }

    #[test]
    fn test_metric_breakdown() {
        use crate::endpoints::chat::MetricBreakdown;
        
        // Test price as floats
        let price = MetricBreakdown {
            input: 0.001,
            output: 0.002,
            total: 0.003,
        };
        let json = serde_json::to_string(&price).unwrap();
        assert!(json.contains("0.001"));
        
        // Test words as floats (even though they're integers in the API)
        let words = MetricBreakdown {
            input: 100.0,
            output: 200.0,
            total: 300.0,
        };
        let json = serde_json::to_string(&words).unwrap();
        assert!(json.contains("100"));
        
        // Test deserialization from integers (as the API sends for words)
        let json_int = r#"{"input":100,"output":200,"total":300}"#;
        let parsed: MetricBreakdown = serde_json::from_str(json_int).unwrap();
        assert_eq!(parsed.input, 100.0);
        assert_eq!(parsed.output, 200.0);
        assert_eq!(parsed.total, 300.0);
    }

    #[test]
    fn test_chat_choice_methods() {
        let choice = create_test_chat_choice();

        assert!(!choice.finished_with_tool_calls());
        assert_eq!(
            choice.content_string(),
            Some("Test response content".to_string())
        );
    }

    #[test]
    fn test_chat_response_content_methods() {
        let text_content = ChatResponseContent::Text("Hello".to_string());
        assert_eq!(text_content.to_string(), "Hello");
        assert!(!text_content.is_empty());

        let empty_content = ChatResponseContent::Text("".to_string());
        assert!(empty_content.is_empty());

        let array_content = ChatResponseContent::Array(vec![
            crate::endpoints::chat::ChatContentObject {
                content_type: "text".to_string(),
                text: "Hello ".to_string(),
            },
            crate::endpoints::chat::ChatContentObject {
                content_type: "text".to_string(),
                text: "world".to_string(),
            },
        ]);
        assert_eq!(array_content.to_string(), "Hello world");
    }

    // Helper functions for creating test data
    fn create_test_chat_response() -> ChatResponse {
        ChatResponse {
            id: "test-id".to_string(),
            model: "test-model".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            choices: vec![create_test_chat_choice()],
            usage: ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            price: Default::default(),
            words: Default::default(),
        }
    }

    fn create_test_chat_choice() -> ChatChoice {
        ChatChoice {
            message: Message {
                role: "assistant".to_string(),
                content: Some(ChatResponseContent::Text(
                    "Test response content".to_string(),
                )),
                tool_calls: None,
            },
            finish_reason: "stop".to_string(),
            index: 0,
        }
    }
}
