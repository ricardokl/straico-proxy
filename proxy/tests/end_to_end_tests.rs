use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test, web, App,
};
use serde_json::json;
use straico_client::client::StraicoClient;
use straico_proxy::{config::ProxyConfig, server, server::AppState};

/// Creates a test AppState for end-to-end tests
fn create_test_app_state() -> AppState {
    AppState {
        client: StraicoClient::new(),
        key: "test-api-key".to_string(),
        config: ProxyConfig::default(),
        print_request_raw: false,
        print_request_converted: false,
        print_response_raw: false,
        print_response_converted: false,
    }
}

async fn create_test_app(
) -> impl Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await
}

#[actix_web::test]
async fn test_chat_completions_endpoint_routing() {
    let app = create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_server_error() || resp.status().is_client_error());
    assert_ne!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_legacy_completion_endpoint_routing() {
    let app = create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/v1/completions")
        .set_json(json!({
            "model": "text-davinci-003",
            "prompt": "Hello",
            "max_tokens": 5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_content_format_validation() {
    let app = create_test_app().await;

    let string_content_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello world"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, string_content_req).await;
    assert!(resp.status().is_server_error() || resp.status().is_success());

    let array_content_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(json!({
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

    let resp = test::call_service(&app, array_content_req).await;
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_openai_compatibility() {
    let app = create_test_app().await;

    let openai_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello!"},
                {"role": "assistant", "content": "Hi there!"},
                {"role": "user", "content": "How are you?"}
            ],
            "temperature": 0.7,
            "max_tokens": 150,
            "stream": false
        }))
        .to_request();

    let resp = test::call_service(&app, openai_req).await;

    assert!(resp.status().is_server_error() || resp.status().is_success());
    assert_ne!(resp.status(), StatusCode::BAD_REQUEST);
}
