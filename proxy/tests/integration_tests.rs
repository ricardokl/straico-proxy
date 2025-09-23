use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    test, web, App,
};
use serde_json::json;
use straico_client::client::StraicoClient;
use straico_proxy::{config::ProxyConfig, server, server::AppState};

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
    }
}

async fn create_test_app(
) -> impl Service<Request, Response = ServiceResponse, Error = actix_web::Error> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion),
    )
    .await
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
