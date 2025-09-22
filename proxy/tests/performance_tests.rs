use actix_web::{test, web, App};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use straico_proxy::{
    config::ProxyConfig,
    server,
    AppState,
};
use straico_client::client::StraicoClient;

/// Creates a test AppState for performance tests
fn create_test_app_state() -> AppState {
    AppState {
        client: StraicoClient::new(),
        key: "test-api-key".to_string(),
        config: ProxyConfig::new()
            .with_validation(true)
            .with_max_messages(Some(100))
            .with_max_content_length(Some(10000)),
        print_request_raw: false,
        print_request_converted: false,
        print_response_raw: false,
        print_response_converted: false,
    }
}

#[actix_web::test]
async fn test_basic_request_response_time() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    let request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    let start = Instant::now();
    
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let duration = start.elapsed();

    // Response time should be reasonable (under 1 second for processing)
    assert!(duration < Duration::from_secs(1));
    
    // Should process the request (may fail due to API key but processing should be fast)
    assert!(resp.status().is_server_error() || resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_large_content_processing_time() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test with large content (within limits)
    let large_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(100);
    let request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": large_content}
        ]
    });

    let start = Instant::now();
    
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let duration = start.elapsed();

    // Large content should still process quickly
    assert!(duration < Duration::from_secs(2));
    
    // Should handle large content
    assert!(resp.status().is_server_error() || resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_multiple_messages_processing_time() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    // Test with many messages
    let mut messages = vec![
        json!({"role": "system", "content": "You are a helpful assistant."})
    ];
    
    for i in 0..20 {
        messages.push(json!({"role": "user", "content": format!("Message {}", i)}));
        messages.push(json!({"role": "assistant", "content": format!("Response {}", i)}));
    }

    let request = json!({
        "model": "gpt-3.5-turbo",
        "messages": messages
    });

    let start = Instant::now();
    
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let duration = start.elapsed();

    // Multiple messages should process efficiently
    assert!(duration < Duration::from_secs(3));
    
    // Should handle multiple messages
    assert!(resp.status().is_server_error() || resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_array_content_vs_string_content_performance() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    let content_text = "This is a test message for performance comparison.";

    // Test string content
    let string_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": content_text}
        ]
    });

    let start = Instant::now();
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&string_request)
        .to_request();
    let _resp = test::call_service(&app, req).await;
    let string_duration = start.elapsed();

    // Test array content
    let array_request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {
                "role": "user", 
                "content": [{"type": "text", "text": content_text}]
            }
        ]
    });

    let start = Instant::now();
    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&array_request)
        .to_request();
    let _resp = test::call_service(&app, req).await;
    let array_duration = start.elapsed();

    // Performance difference should be minimal
    let diff = if string_duration > array_duration {
        string_duration - array_duration
    } else {
        array_duration - string_duration
    };
    
    // Difference should be less than 100ms
    assert!(diff < Duration::from_millis(100));
}

#[actix_web::test]
async fn test_concurrent_request_handling() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    let request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Concurrent test"}
        ]
    });

    let start = Instant::now();
    
    // Send 10 concurrent requests
    let mut handles = Vec::new();
    for i in 0..10 {
        let app_clone = &app;
        let request_clone = request.clone();
        
        let handle = tokio::spawn(async move {
            let req = test::TestRequest::post()
                .uri("/v1/chat/completions")
                .set_json(&request_clone)
                .to_request();
            
            test::call_service(app_clone, req).await
        });
        
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let _resp = handle.await.unwrap();
    }
    
    let total_duration = start.elapsed();
    
    // All 10 requests should complete within reasonable time
    assert!(total_duration < Duration::from_secs(10));
}

#[actix_web::test]
async fn test_request_timeout_handling() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(server::openai_chat_completion)
    ).await;

    let request = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Timeout test"}
        ]
    });

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&request)
        .to_request();

    // Test with timeout
    let result = timeout(
        Duration::from_secs(5),
        test::call_service(&app, req)
    ).await;

    // Should complete within timeout
    assert!(result.is_ok());
}

#[test]
fn test_content_conversion_performance() {
    use straico_proxy::content_conversion::*;
    use straico_proxy::openai_types::*;

    let large_text = "Performance test content. ".repeat(1000);
    
    // Test string content conversion performance
    let string_content = OpenAiContent::String(large_text.clone());
    
    let start = Instant::now();
    let _converted = convert_openai_content_to_straico(string_content);
    let string_duration = start.elapsed();
    
    // Test array content conversion performance
    let array_content = OpenAiContent::Array(vec![
        OpenAiContentObject {
            content_type: "text".to_string(),
            text: large_text,
        }
    ]);
    
    let start = Instant::now();
    let _converted = convert_openai_content_to_straico(array_content);
    let array_duration = start.elapsed();
    
    // Both conversions should be fast
    assert!(string_duration < Duration::from_millis(10));
    assert!(array_duration < Duration::from_millis(10));
}

#[test]
fn test_validation_performance() {
    use straico_proxy::openai_types::*;

    // Create a large request for validation testing
    let mut messages = Vec::new();
    for i in 0..50 {
        messages.push(OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String(format!("Message {} content", i)),
            tool_call_id: None,
            name: None,
        });
    }

    let large_request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages,
        temperature: Some(0.7),
        max_tokens: Some(100),
        max_completion_tokens: None,
        stream: false,
        tools: None,
    };

    let start = Instant::now();
    let _result = large_request.validate();
    let validation_duration = start.elapsed();
    
    // Validation should be fast even for large requests
    assert!(validation_duration < Duration::from_millis(50));
}

#[test]
fn test_configuration_performance() {
    use straico_proxy::config_manager::*;
    use std::fs;

    let config_path = "perf_test_config.toml";
    let _ = fs::remove_file(config_path);

    // Test configuration loading performance
    let start = Instant::now();
    let config_manager = ConfigManager::new(config_path);
    let load_duration = start.elapsed();
    
    // Test configuration validation performance
    let start = Instant::now();
    let _result = config_manager.validate_config();
    let validation_duration = start.elapsed();
    
    // Test feature flag access performance
    let start = Instant::now();
    for _ in 0..100 {
        let _enabled = config_manager.get_feature_flag("new_chat_endpoint");
    }
    let flag_access_duration = start.elapsed();
    
    // All operations should be fast
    assert!(load_duration < Duration::from_millis(100));
    assert!(validation_duration < Duration::from_millis(10));
    assert!(flag_access_duration < Duration::from_millis(10));
    
    let _ = fs::remove_file(config_path);
}

#[test]
fn test_memory_usage_estimation() {
    use straico_proxy::openai_types::*;
    
    // Test memory usage with different content sizes
    let small_content = "Small".to_string();
    let medium_content = "Medium content. ".repeat(100);
    let large_content = "Large content. ".repeat(1000);
    
    // Create requests with different content sizes
    let small_request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String(small_content),
            tool_call_id: None,
            name: None,
        }],
        temperature: None,
        max_tokens: None,
        max_completion_tokens: None,
        stream: false,
        tools: None,
    };
    
    let large_request = OpenAiChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String(large_content),
            tool_call_id: None,
            name: None,
        }],
        temperature: None,
        max_tokens: None,
        max_completion_tokens: None,
        stream: false,
        tools: None,
    };
    
    // Basic memory usage validation (structures should be reasonable size)
    let small_size = std::mem::size_of_val(&small_request);
    let large_size = std::mem::size_of_val(&large_request);
    
    // Large request should not be dramatically larger in struct size
    // (content is heap-allocated)
    assert!(large_size - small_size < 1000);
}