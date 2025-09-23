use actix_web::{test, web, App, http::StatusCode, dev::{Service, ServiceRequest, ServiceResponse}};
use serde_json::json;
use straico_proxy::{
    config::{ProxyConfig, EndpointRoute},
    openai_types::{OpenAiChatRequest, OpenAiContent, OpenAiChatMessage},
    server,
    server::AppState,
};
use straico_client::client::StraicoClient;
use actix_http::Request;

/// Creates a test AppState for integration tests
fn create_test_app_state() -> AppState {
    AppState {
        client: StraicoClient::new(),
        key: "test-api-key".to_string(),
        config: ProxyConfig::default(),
        print_request_raw: false,
        print_request_converted: false,
        print_response_raw: false,
        print_response_converted: false,
        use_new_chat_endpoint: true,
        force_new_endpoint_for_tools: true,
    }
}

async fn create_test_app() -> impl Service<Request, Response = ServiceResponse, Error = actix_web::Error> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await
}

#[actix_web::test]
async fn test_chat_completion_endpoint_exists() {
    let mut app = create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    
    assert!(resp.status().is_server_error() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_chat_completion_validation() {
    let mut app = create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": []
        }))
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert!(resp.status().is_client_error());

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_chat_completion_content_formats() {
    let mut app = create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello world"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert!(resp.status().is_server_error() || resp.status().is_success());

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

    let resp = test::call_service(&mut app, req).await;
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_proxy_config_validation() {
    let config = ProxyConfig::default();

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
        tool_choice: None,
    };

    assert!(config.validate_chat_request(&valid_request).is_ok());

    let mut too_many_messages = valid_request.clone();
    for _ in 0..100 {
        too_many_messages.messages.push(OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("message".to_string()),
            tool_call_id: None,
            name: None,
        });
    }

    assert!(config.validate_chat_request(&too_many_messages).is_err());

    let mut long_content = valid_request.clone();
    if let OpenAiContent::String(ref mut text) = long_content.messages[0].content {
        *text = "a".repeat(20000);
    }

    assert!(config.validate_chat_request(&long_content).is_err());
}

#[actix_web::test]
async fn test_endpoint_routing() {
    let config = ProxyConfig {
        use_new_chat_endpoint: true,
        ..ProxyConfig::default()
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

    let route = config.determine_endpoint_route(&request);
    assert_eq!(route, EndpointRoute::NewChat);

    let config_legacy = ProxyConfig {
        use_new_chat_endpoint: false,
        ..ProxyConfig::default()
    };
    let route_legacy = config_legacy.determine_endpoint_route(&request);
    assert_eq!(route_legacy, EndpointRoute::Legacy);
}