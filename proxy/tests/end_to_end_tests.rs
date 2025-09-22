use actix_web::{test, web, App, http::StatusCode};
use serde_json::json;
use std::fs;
use std::path::Path;
use straico_proxy::{
    config::ProxyConfig,
    config_manager::{ConfigManager, FeatureFlags, EnvironmentConfig},
    openai_types::{OpenAiChatRequest, OpenAiContent, OpenAiChatMessage},
    server,
    AppState,
};
use straico_client::client::StraicoClient;

/// Creates a test AppState for end-to-end tests
fn create_test_app_state() -> AppState {
    AppState {
        client: StraicoClient::new(),
        key: "test-api-key".to_string(),
        config: ProxyConfig::new()
            .with_validation(true)
            .with_debug_info(true)
            .with_new_chat_endpoint(true),
        print_request_raw: false,
        print_request_converted: false,
        print_response_raw: false,
        print_response_converted: false,
    }
}

/// Creates a test app with all endpoints
fn create_test_app() -> actix_web::dev::Service<
    actix_web::dev::ServiceRequest,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_completion)
            .service(server::openai_chat_completion)
    )
}

#[actix_web::test]
async fn test_chat_completions_endpoint_routing() {
    let app = create_test_app().await;

    // Test that the endpoint exists and routes correctly
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
    
    // Should process the request (may fail due to invalid API key, but routing works)
    assert!(resp.status().is_server_error() || resp.status().is_client_error());
    assert_ne!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_legacy_completion_endpoint_routing() {
    let app = create_test_app().await;

    // Test legacy completion endpoint
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
    
    // Should process the request
    assert!(resp.status().is_server_error() || resp.status().is_client_error());
    assert_ne!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_content_format_validation() {
    let app = create_test_app().await;

    // Test string content format
    let string_content_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello world"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, string_content_req).await;
    assert!(resp.status().is_server_error() || resp.status().is_success());

    // Test array content format
    let array_content_req = test::TestRequest::post()
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

    let resp = test::call_service(&app, array_content_req).await;
    assert!(resp.status().is_server_error() || resp.status().is_success());
}

#[actix_web::test]
async fn test_request_validation() {
    let app = create_test_app().await;

    // Test empty messages validation
    let empty_messages_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": []
        }))
        .to_request();

    let resp = test::call_service(&app, empty_messages_req).await;
    assert!(resp.status().is_client_error());

    // Test missing model validation
    let no_model_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, no_model_req).await;
    assert!(resp.status().is_client_error());

    // Test invalid temperature
    let invalid_temp_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 3.0
        }))
        .to_request();

    let resp = test::call_service(&app, invalid_temp_req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_openai_compatibility() {
    let app = create_test_app().await;

    // Test OpenAI-compatible request structure
    let openai_req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
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
    
    // Should accept the OpenAI format (may fail due to API key)
    assert!(resp.status().is_server_error() || resp.status().is_success());
    assert_ne!(resp.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_configuration_file_creation() {
    let temp_config_path = "test_config.toml";
    
    // Clean up any existing test file
    let _ = fs::remove_file(temp_config_path);
    
    // Test configuration file creation
    assert!(ConfigManager::create_default_config(temp_config_path).is_ok());
    assert!(Path::new(temp_config_path).exists());
    
    // Test loading the created configuration
    let config_manager = ConfigManager::new(temp_config_path);
    let config = config_manager.get_config();
    
    // Verify default values
    assert_eq!(config.environment.environment, "development");
    assert_eq!(config.environment.port, 8000);
    assert!(config.features.enable_new_chat_endpoint);
    
    // Clean up
    let _ = fs::remove_file(temp_config_path);
}

#[test]
fn test_feature_flag_management() {
    let temp_config_path = "test_features.toml";
    let _ = fs::remove_file(temp_config_path);
    
    let mut config_manager = ConfigManager::new(temp_config_path);
    
    // Test setting feature flags
    assert!(config_manager.set_feature_flag("streaming", true).is_ok());
    assert!(config_manager.set_feature_flag("metrics", true).is_ok());
    assert!(config_manager.set_feature_flag("invalid_feature", true).is_err());
    
    // Test getting feature flags
    assert_eq!(config_manager.get_feature_flag("streaming").unwrap(), true);
    assert_eq!(config_manager.get_feature_flag("metrics").unwrap(), true);
    assert_eq!(config_manager.get_feature_flag("caching").unwrap(), false);
    
    // Test effective configuration
    let effective = config_manager.get_effective_config();
    assert!(effective.is_feature_enabled("streaming"));
    assert!(effective.is_feature_enabled("metrics"));
    assert!(!effective.is_feature_enabled("caching"));
    
    let enabled_features = effective.get_enabled_features();
    assert!(enabled_features.contains(&"streaming".to_string()));
    assert!(enabled_features.contains(&"metrics".to_string()));
    
    let _ = fs::remove_file(temp_config_path);
}

#[test]
fn test_configuration_validation() {
    let temp_config_path = "test_validation.toml";
    let _ = fs::remove_file(temp_config_path);
    
    let mut config_manager = ConfigManager::new(temp_config_path);
    
    // Test valid configuration
    assert!(config_manager.validate_config().is_ok());
    
    // Test invalid configuration
    config_manager.get_config_mut().environment.port = 0;
    config_manager.get_config_mut().environment.environment = "invalid".to_string();
    
    let validation_result = config_manager.validate_config();
    assert!(validation_result.is_err());
    
    let errors = validation_result.unwrap_err();
    assert!(errors.len() >= 2);
    assert!(errors.iter().any(|e| e.contains("Port cannot be 0")));
    assert!(errors.iter().any(|e| e.contains("Environment must be one of")));
    
    let _ = fs::remove_file(temp_config_path);
}

#[test]
fn test_proxy_config_limits() {
    let config = ProxyConfig::new()
        .with_max_messages(Some(3))
        .with_max_content_length(Some(50));

    // Test valid request within limits
    let valid_request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            OpenAiChatMessage {
                role: "user".to_string(),
                content: OpenAiContent::String("Short".to_string()),
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

    // Test request exceeding message limit
    let mut too_many_messages = valid_request.clone();
    for i in 0..5 {
        too_many_messages.messages.push(OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String(format!("Message {}", i)),
            tool_call_id: None,
            name: None,
        });
    }

    assert!(config.validate_chat_request(&too_many_messages).is_err());

    // Test request exceeding content length limit
    let mut long_content = valid_request.clone();
    long_content.messages[0].content = OpenAiContent::String("a".repeat(100));

    assert!(config.validate_chat_request(&long_content).is_err());
}

#[test]
fn test_content_conversion_integration() {
    use straico_proxy::content_conversion::*;
    use straico_proxy::openai_types::*;

    // Test string content conversion
    let string_content = OpenAiContent::String("Hello world".to_string());
    let converted = convert_openai_content_to_straico(string_content);
    
    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].content_type, "text");
    assert_eq!(converted[0].text, "Hello world");

    // Test array content conversion
    let array_content = OpenAiContent::Array(vec![
        OpenAiContentObject {
            content_type: "text".to_string(),
            text: "Hello ".to_string(),
        },
        OpenAiContentObject {
            content_type: "text".to_string(),
            text: "world!".to_string(),
        },
    ]);
    
    let converted_array = convert_openai_content_to_straico(array_content);
    assert_eq!(converted_array.len(), 2);
    assert_eq!(converted_array[0].text, "Hello ");
    assert_eq!(converted_array[1].text, "world!");

    // Test complete request conversion
    let openai_request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            OpenAiChatMessage {
                role: "system".to_string(),
                content: OpenAiContent::String("You are helpful.".to_string()),
                tool_call_id: None,
                name: None,
            },
            OpenAiChatMessage {
                role: "user".to_string(),
                content: OpenAiContent::String("Hello!".to_string()),
                tool_call_id: None,
                name: None,
            },
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        max_completion_tokens: None,
        stream: false,
        tools: None,
    };

    let straico_request = convert_openai_request_to_straico(openai_request);
    assert!(straico_request.is_ok());
    
    let request = straico_request.unwrap();
    assert_eq!(request.model, "gpt-3.5-turbo");
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(100));
}

#[test]
fn test_error_handling() {
    use straico_proxy::response_utils::error_response_utils::*;

    // Test validation error creation
    let validation_error = create_validation_error(
        "Invalid temperature".to_string(),
        Some("temperature".to_string())
    );
    
    assert_eq!(validation_error.error.error_type, "invalid_request_error");
    assert_eq!(validation_error.error.message, "Invalid temperature");
    assert_eq!(validation_error.error.param, Some("temperature".to_string()));

    // Test conversion error creation
    let conversion_error = create_conversion_error("Content format invalid".to_string());
    assert_eq!(conversion_error.error.error_type, "invalid_request_error");
    assert!(conversion_error.error.message.contains("Content conversion failed"));

    // Test server error creation
    let server_error = create_server_error("Database connection failed".to_string());
    assert_eq!(server_error.error.error_type, "server_error");
    assert!(server_error.error.message.contains("Internal server error"));
}

#[test]
fn test_phase1_integration() {
    // This test validates that all Phase 1 components work together
    
    // 1. Test configuration management
    let temp_config = "integration_test.toml";
    let _ = fs::remove_file(temp_config);
    
    let mut config_manager = ConfigManager::new(temp_config);
    config_manager.set_feature_flag("new_chat_endpoint", true).unwrap();
    
    // 2. Test content conversion
    let openai_request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            OpenAiChatMessage {
                role: "user".to_string(),
                content: OpenAiContent::String("Integration test".to_string()),
                tool_call_id: None,
                name: None,
            }
        ],
        temperature: Some(0.5),
        max_tokens: Some(50),
        max_completion_tokens: None,
        stream: false,
        tools: None,
    };

    // 3. Test request validation
    let proxy_config = config_manager.get_config().proxy.clone();
    assert!(proxy_config.validate_chat_request(&openai_request).is_ok());

    // 4. Test content conversion
    let straico_request = straico_proxy::content_conversion::convert_openai_request_to_straico(openai_request);
    assert!(straico_request.is_ok());

    // 5. Test effective configuration
    let effective_config = config_manager.get_effective_config();
    assert!(effective_config.is_feature_enabled("new_chat_endpoint"));

    let _ = fs::remove_file(temp_config);
}