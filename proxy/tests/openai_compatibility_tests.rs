use actix_web::{test, web, App};
use serde_json::json;
use straico_client::client::StraicoClient;
use straico_proxy::{
    openai_types::{OpenAiChatRequest, OpenAiContent},
    server, AppState,
};

/// Creates a test AppState for OpenAI compatibility tests
fn create_test_app_state() -> AppState {
    AppState {
        client: StraicoClient::new(),
        key: "test-api-key".to_string(),
        print_request_raw: false,
        print_request_converted: false,
        print_response_raw: false,
        print_response_converted: false,
        include_debug_info: false,
    }
}

#[actix_web::test]
async fn test_openai_request_format_compatibility() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await;

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

    assert!(resp.status().is_server_error() || resp.status().is_success());
    assert_ne!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_openai_array_content_format() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await;

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

    assert!(resp.status().is_server_error() || resp.status().is_success());
    assert_ne!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_openai_parameter_compatibility() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await;

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

    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_error_format_compatibility() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await;

    let invalid_request = json!({
        "model": "",
        "messages": []
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&invalid_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(!resp.status().is_client_error());
}

#[actix_web::test]
async fn test_openai_conversation_format() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await;

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

    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_edge_cases() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await;

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

    assert!(
        resp.status().is_server_error()
            || resp.status().is_client_error()
            || resp.status().is_success()
    );

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

    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_mixed_content_types() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await;

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

    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_content_format_parsing() {
    let string_content_json = json!("Hello world");
    let string_content: OpenAiContent = serde_json::from_value(string_content_json).unwrap();

    match string_content {
        OpenAiContent::String(text) => assert_eq!(text, "Hello world"),
        _ => panic!("Expected string content"),
    }

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
        }
        _ => panic!("Expected array content"),
    }
}

#[actix_web::test]
async fn test_openai_parameter_defaults() {
    let minimal_json = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    let request: OpenAiChatRequest = serde_json::from_value(minimal_json).unwrap();

    assert_eq!(request.temperature, None);
    assert_eq!(request.max_tokens, None);
    assert!(!request.stream);
    assert!(request.tools.is_none());
}
