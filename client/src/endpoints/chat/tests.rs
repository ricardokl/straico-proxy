#[cfg(test)]
mod tests {

    use crate::endpoints::chat::{
        ChatChoice, ChatMessage, ChatRequest, ChatResponse, ChatResponseContent, ChatUsage,
        ContentObject, Message,
    };

    #[test]
    fn test_chat_message_convenience_methods() {
        let system_msg = ChatMessage::system("System prompt");
        assert_eq!(system_msg.role, "system");
        assert_eq!(system_msg.content[0].text, "System prompt");

        let user_msg = ChatMessage::user("User input");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content[0].text, "User input");

        let assistant_msg = ChatMessage::assistant("Assistant response");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content[0].text, "Assistant response");

        let tool_msg = ChatMessage::tool("Tool output");
        assert_eq!(tool_msg.role, "tool");
        assert_eq!(tool_msg.content[0].text, "Tool output");
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
            choices: vec![create_test_chat_choice()],
            model: "test-model".to_string(),
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            id: Some("test-id".to_string()),
            object: Some("chat.completion".to_string()),
            created: Some(1234567890),
            tools: None,
            tool_choice: None,
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
            index: Some(0),
        }
    }
}
