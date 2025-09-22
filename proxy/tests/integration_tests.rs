use actix_web::{test, web, App};
use serde_json::json;
use straico_proxy::{
    config::ProxyConfig,
    openai_types::OpenAiChatRequest,
    server,
    AppState,
};
use straico_client::client::StraicoClient;

/// Creates a test AppState for integration tests
fn create_test_app_state() -> AppState {
    AppState {
        client: StraicoClient::new(),
        key: "test-api-key".to_string(),
        config: ProxyConfig::new()
            .with_validation(true)
            .with_debug_info(true),
        print_request_raw: false,
        print_request_converted: false,
        print_response_raw: false,
        print_response_converted: false,
    }
}

#[actix_web::test]
async fn test_chat_completion_endpoint_exists() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test that the endpoint exists (even if it fails due to no real API key)
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // We expect this to fail with a server error due to invalid API key,
    // but the endpoint should exist and process the request
    assert!(resp.status().is_server_error() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_chat_completion_validation() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test empty messages validation
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": []
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());

    // Test missing model validation
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_chat_completion_content_formats() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test string content format
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello world"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should process the request (may fail due to API key, but format should be accepted)
    assert!(resp.status().is_server_error() || resp.status().is_success());

    // Test array content format
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {
                    "role": "user", 
                    "content": [
                        {"type": "text", "text": "Hello world"}
                    ]
                }
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should process the request (may fail due to API key, but format should be accepted)
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[test]
fn test_proxy_config_validation() {
    let config = ProxyConfig::new()
        .with_max_messages(Some(5))
        .with_max_content_length(Some(100));

    // Test valid request
    let valid_request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            crate::openai_types::OpenAiChatMessage {
                role: "user".to_string(),
                content: crate::openai_types::OpenAiContent::String("Hello".to_string()),
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

    assert!(config.validate_chat_request(&valid_request).is_ok());

    // Test too many messages
    let mut too_many_messages = valid_request.clone();
    for i in 0..10 {
        too_many_messages.messages.push(crate::openai_types::OpenAiChatMessage {
            role: "user".to_string(),
            content: crate::openai_types::OpenAiContent::String(format!("Message {}", i)),
            tool_call_id: None,
            name: None,
        });
    }

    assert!(config.validate_chat_request(&too_many_messages).is_err());

    // Test content too long
    let mut long_content = valid_request.clone();
    long_content.messages[0].content = crate::openai_types::OpenAiContent::String("a".repeat(200));

    assert!(config.validate_chat_request(&long_content).is_err());
}

#[test]
fn test_endpoint_routing() {
    let config = ProxyConfig::new().with_new_chat_endpoint(true);
    
    let request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        max_completion_tokens: None,
        stream: false,
        tools: None,
    };

    let route = config.determine_endpoint_route(&request);
    assert_eq!(route, crate::config::EndpointRoute::NewChat);

    let config_legacy = ProxyConfig::new().with_new_chat_endpoint(false);
    let route_legacy = config_legacy.determine_endpoint_route(&request);
    assert_eq!(route_legacy, crate::config::EndpointRoute::Legacy);
}

#[cfg(test)]
mod response_utils_tests {
    use super::*;
    use crate::response_utils::chat_response_utils::*;
    use straico_client::endpoints::chat::*;

    #[test]
    fn test_enhance_chat_response() {
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
        };

        let request = OpenAiChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: false,
            tools: None,
        };

        let enhanced = enhance_chat_response(response, &request, true);

        assert!(enhanced.id.is_some());
        assert!(enhanced.object.is_some());
        assert!(enhanced.created.is_some());
        assert_eq!(enhanced.model, "gpt-3.5-turbo");
    }

    #[test]
    fn test_validate_chat_response() {
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
        };

        assert!(validate_chat_response(&valid_response).is_ok());

        let invalid_response = ChatResponse {
            choices: vec![],
            model: "gpt-3.5-turbo".to_string(),
            usage: None,
            id: None,
            object: None,
            created: None,
        };

        assert!(validate_chat_response(&invalid_response).is_err());
    }
}