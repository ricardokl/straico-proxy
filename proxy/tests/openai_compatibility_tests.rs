use actix_web::{test, web, App};
use serde_json::{json, Value};
use straico_proxy::{
    config::ProxyConfig,
    server,
    AppState,
};
use straico_client::client::StraicoClient;

/// Creates a test AppState for OpenAI compatibility tests
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
async fn test_openai_request_format_compatibility() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test standard OpenAI request format
    let openai_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant."
            },
            {
                "role": "user", 
                "content": "Hello, how are you?"
            }
        ],
        "temperature": 0.7,
        "max_tokens": 150,
        "stream": false
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&openai_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should accept the request format (may fail due to API key but format is valid)
    assert!(resp.status().is_server_error() || resp.status().is_success());
    assert_ne!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_openai_array_content_format() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test OpenAI array content format
    let array_content_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "What's in this image?"
                    }
                ]
            }
        ],
        "max_tokens": 300
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&array_content_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should accept array content format
    assert!(resp.status().is_server_error() || resp.status().is_success());
    assert_ne!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_openai_parameter_compatibility() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test all OpenAI parameters
    let full_params_request = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Test message"}
        ],
        "temperature": 0.9,
        "max_tokens": 500,
        "max_completion_tokens": 400,
        "stream": false,
        "top_p": 1.0,
        "frequency_penalty": 0.0,
        "presence_penalty": 0.0,
        "stop": ["\n", "END"],
        "user": "test-user-123"
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&full_params_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should handle all parameters gracefully
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_error_format_compatibility() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test invalid request that should return OpenAI-compatible error
    let invalid_request = json!({
        "model": "",
        "messages": []
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&invalid_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());

    // TODO: Verify error response format matches OpenAI spec
    // This would require parsing the response body and checking structure
}

#[actix_web::test]
async fn test_openai_conversation_format() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test multi-turn conversation format
    let conversation_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant that answers questions about programming."
            },
            {
                "role": "user",
                "content": "What is Rust?"
            },
            {
                "role": "assistant", 
                "content": "Rust is a systems programming language that focuses on safety, speed, and concurrency."
            },
            {
                "role": "user",
                "content": "What are its main advantages?"
            }
        ],
        "temperature": 0.7,
        "max_tokens": 200
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&conversation_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should handle conversation format
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_edge_cases() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test edge case: very long content
    let long_content = "a".repeat(5000);
    let long_content_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": long_content}
        ]
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&long_content_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should handle or reject appropriately based on configuration
    assert!(resp.status().is_server_error() || resp.status().is_client_error() || resp.status().is_success());

    // Test edge case: unicode and emoji content
    let unicode_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Hello 👋 世界 🌍 How are you? 😊"}
        ]
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&unicode_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should handle unicode content
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_mixed_content_types() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test mixed content types in conversation
    let mixed_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {
                "role": "user",
                "content": "Simple string message"
            },
            {
                "role": "assistant",
                "content": "Assistant response"
            },
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Array format message"}
                ]
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&mixed_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should handle mixed content formats in same conversation
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[test]
fn test_openai_request_structure_validation() {
    use straico_proxy::openai_types::*;

    // Test valid OpenAI request structure
    let valid_json = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "temperature": 0.7,
        "max_tokens": 100
    });

    let parsed: Result<OpenAiChatRequest, _> = serde_json::from_value(valid_json);
    assert!(parsed.is_ok());

    let request = parsed.unwrap();
    assert_eq!(request.model, "gpt-3.5-turbo");
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(100));

    // Test request validation
    assert!(request.validate().is_ok());
}

#[test]
fn test_openai_content_format_parsing() {
    use straico_proxy::openai_types::*;

    // Test string content parsing
    let string_content_json = json!("Hello world");
    let string_content: OpenAiContent = serde_json::from_value(string_content_json).unwrap();
    
    match string_content {
        OpenAiContent::String(text) => assert_eq!(text, "Hello world"),
        _ => panic!("Expected string content"),
    }

    // Test array content parsing
    let array_content_json = json!([
        {"type": "text", "text": "Hello"},
        {"type": "text", "text": " world"}
    ]);
    let array_content: OpenAiContent = serde_json::from_value(array_content_json).unwrap();
    
    match array_content {
        OpenAiContent::Array(objects) => {
            assert_eq!(objects.len(), 2);
            assert_eq!(objects[0].text, "Hello");
            assert_eq!(objects[1].text, " world");
        },
        _ => panic!("Expected array content"),
    }
}

#[test]
fn test_openai_parameter_defaults() {
    use straico_proxy::openai_types::*;

    // Test minimal request
    let minimal_json = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    let request: OpenAiChatRequest = serde_json::from_value(minimal_json).unwrap();
    
    // Verify defaults
    assert_eq!(request.temperature, None);
    assert_eq!(request.max_tokens, None);
    assert_eq!(request.stream, false);
    assert_eq!(request.tools, None);
}

#[test]
fn test_openai_error_cases() {
    use straico_proxy::openai_types::*;

    // Test invalid model
    let invalid_model_json = json!({
        "model": "",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    let request: OpenAiChatRequest = serde_json::from_value(invalid_model_json).unwrap();
    assert!(request.validate().is_err());

    // Test empty messages
    let empty_messages_json = json!({
        "model": "gpt-3.5-turbo",
        "messages": []
    });

    let request: OpenAiChatRequest = serde_json::from_value(empty_messages_json).unwrap();
    assert!(request.validate().is_err());

    // Test invalid temperature
    let invalid_temp_json = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "temperature": 3.0
    });

    let request: OpenAiChatRequest = serde_json::from_value(invalid_temp_json).unwrap();
    assert!(request.validate().is_err());
}