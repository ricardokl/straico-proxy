#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        StraicoClient,
        endpoints::chat::{
            ChatRequest, ChatMessage, ContentObject, ChatResponse, ChatChoice,
            ChatResponseMessage, ChatResponseContent, ChatUsage,
            builders::*, response_utils::*, ChatClientExt,
        },
    };

    #[test]
    fn test_chat_client_extension() {
        let client = StraicoClient::new();
        
        // Test that the chat_completions method exists and returns the right type
        let _builder = client.chat_completions();
        // If this compiles, the extension trait is working
    }

    #[test]
    fn test_simple_chat_request_builder() {
        let request = simple_chat_request("gpt-3.5-turbo", "Hello world");
        
        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.messages[0].content[0].text, "Hello world");
    }

    #[test]
    fn test_system_user_chat_request_builder() {
        let request = system_user_chat_request(
            "gpt-4",
            "You are helpful",
            "What is Rust?"
        );
        
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, "system");
        assert_eq!(request.messages[0].content[0].text, "You are helpful");
        assert_eq!(request.messages[1].role, "user");
        assert_eq!(request.messages[1].content[0].text, "What is Rust?");
    }

    #[test]
    fn test_conversation_chat_request_builder() {
        let messages = vec![
            ChatMessage::system("System message"),
            ChatMessage::user("User message"),
            ChatMessage::assistant("Assistant message"),
        ];
        
        let request = conversation_chat_request("claude-3", messages.clone());
        
        assert_eq!(request.model, "claude-3");
        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, "system");
        assert_eq!(request.messages[1].role, "user");
        assert_eq!(request.messages[2].role, "assistant");
    }

    #[test]
    fn test_advanced_chat_request_builder() {
        let messages = vec![
            ChatMessage::user("Test message"),
        ];
        
        let request = advanced_chat_request(
            "gpt-3.5-turbo",
            messages,
            Some(0.8),
            Some(150)
        );
        
        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.temperature, Some(0.8));
        assert_eq!(request.max_tokens, Some(150));
    }

    #[test]
    fn test_advanced_chat_request_builder_no_params() {
        let messages = vec![
            ChatMessage::user("Test message"),
        ];
        
        let request = advanced_chat_request(
            "gpt-3.5-turbo",
            messages,
            None,
            None
        );
        
        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.temperature, None);
        assert_eq!(request.max_tokens, None);
    }

    #[test]
    fn test_response_utils_get_first_content() {
        let response = create_test_chat_response();
        let content = get_first_content(&response);
        
        assert_eq!(content, Some("Test response content".to_string()));
    }

    #[test]
    fn test_response_utils_has_tool_calls() {
        let response = create_test_chat_response();
        assert!(!has_tool_calls(&response));
        
        let response_with_tools = create_test_chat_response_with_tools();
        assert!(has_tool_calls(&response_with_tools));
    }

    #[test]
    fn test_response_utils_get_all_contents() {
        let response = create_test_chat_response_multiple_choices();
        let contents = get_all_contents(&response);
        
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0], "First choice content");
        assert_eq!(contents[1], "Second choice content");
    }

    #[test]
    fn test_response_utils_get_finish_reason() {
        let response = create_test_chat_response();
        let reason = get_finish_reason(&response);
        
        assert_eq!(reason, Some("stop"));
    }

    #[test]
    fn test_response_utils_was_truncated() {
        let response = create_test_chat_response();
        assert!(!was_truncated(&response));
        
        let truncated_response = create_test_chat_response_truncated();
        assert!(was_truncated(&truncated_response));
    }

    #[test]
    fn test_response_utils_get_usage() {
        let response = create_test_chat_response();
        let usage = get_usage(&response);
        
        assert!(usage.is_some());
        let usage = usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

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
        let request = ChatRequest::new()
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
        assert_eq!(response.first_content(), Some("Test response content".to_string()));
        assert!(!response.has_tool_calls());
    }

    #[test]
    fn test_chat_choice_methods() {
        let choice = create_test_chat_choice();
        
        assert!(!choice.finished_with_tool_calls());
        assert_eq!(choice.content_string(), Some("Test response content".to_string()));
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
        }
    }

    fn create_test_chat_response_with_tools() -> ChatResponse {
        let mut response = create_test_chat_response();
        response.choices[0].message.tool_calls = Some(vec![
            crate::endpoints::chat::ChatToolCall {
                id: "tool-1".to_string(),
                function: crate::endpoints::chat::ChatFunctionCall {
                    name: "test_function".to_string(),
                    arguments: "{}".to_string(),
                },
                tool_type: "function".to_string(),
            }
        ]);
        response
    }

    fn create_test_chat_response_multiple_choices() -> ChatResponse {
        ChatResponse {
            choices: vec![
                ChatChoice {
                    message: ChatResponseMessage {
                        role: "assistant".to_string(),
                        content: Some(ChatResponseContent::Text("First choice content".to_string())),
                        tool_calls: None,
                    },
                    finish_reason: "stop".to_string(),
                    index: Some(0),
                },
                ChatChoice {
                    message: ChatResponseMessage {
                        role: "assistant".to_string(),
                        content: Some(ChatResponseContent::Text("Second choice content".to_string())),
                        tool_calls: None,
                    },
                    finish_reason: "stop".to_string(),
                    index: Some(1),
                },
            ],
            model: "test-model".to_string(),
            usage: None,
            id: None,
            object: None,
            created: None,
        }
    }

    fn create_test_chat_response_truncated() -> ChatResponse {
        let mut response = create_test_chat_response();
        response.choices[0].finish_reason = "length".to_string();
        response
    }

    fn create_test_chat_choice() -> ChatChoice {
        ChatChoice {
            message: ChatResponseMessage {
                role: "assistant".to_string(),
                content: Some(ChatResponseContent::Text("Test response content".to_string())),
                tool_calls: None,
            },
            finish_reason: "stop".to_string(),
            index: Some(0),
        }
    }
}