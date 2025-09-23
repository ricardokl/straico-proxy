#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ProxyConfig,
        openai_types::{OpenAiChatRequest, OpenAiChatMessage, OpenAiContent},
    };
    use straico_client::endpoints::chat::{ChatResponse, ChatChoice, ChatResponseMessage, ChatResponseContent};

    #[actix_web::test]
    async fn test_enhance_chat_response() {
        let mut response = ChatResponse {
            choices: vec![
                ChatChoice {
                    message: ChatResponseMessage {
                        role: "assistant".to_string(),
                        content: Some(ChatResponseContent::Text("Hello".to_string())),
                        tool_calls: None,
                    },
                    finish_reason: "stop".to_string(),
                    index: Some(0),
                }
            ],
            model: "".to_string(),
            usage: None,
            id: None,
            object: None,
            created: None,
            tool_choice: None,
            tools: None,
        };

        let request = OpenAiChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        };

        let enhanced = enhance_chat_response(response, &request, true);

        assert!(enhanced.id.is_some());
        assert!(enhanced.object.is_some());
        assert!(enhanced.created.is_some());
        assert_eq!(enhanced.model, "gpt-3.5-turbo");
    }

    #[actix_web::test]
    async fn test_validate_chat_response() {
        let valid_response = ChatResponse {
            choices: vec![
                ChatChoice {
                    message: ChatResponseMessage {
                        role: "assistant".to_string(),
                        content: Some(ChatResponseContent::Text("Hello".to_string())),
                        tool_calls: None,
                    },
                    finish_reason: "stop".to_string(),
                    index: Some(0),
                }
            ],
            model: "gpt-3.5-turbo".to_string(),
            usage: None,
            id: None,
            object: None,
            created: None,
            tool_choice: None,
            tools: None,
        };

        assert!(validate_chat_response(&valid_response).is_ok());

        let invalid_response = ChatResponse {
            choices: vec![],
            model: "gpt-3.5-turbo".to_string(),
            usage: None,
            id: None,
            object: None,
            created: None,
            tool_choice: None,
            tools: None,
        };

        assert!(validate_chat_response(&invalid_response).is_err());
    }
}
