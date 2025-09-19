# P3-T6: Testing and Validation (Phase 3)

## Objective
Thoroughly test the complete streaming implementation including detection, fallback mechanisms, error handling, and end-to-end streaming functionality.

## Background
Phase 3 completes the streaming implementation. This final testing phase ensures all streaming scenarios work correctly, fallbacks are reliable, and the system is production-ready.

## Tasks

### 1. Unit Tests for Streaming Components
**File**: `proxy/src/streaming_detection.rs` (extend with tests)

**Detection Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_streaming_detection_cache() {
        let cache = StreamingCapabilityCache::new(Duration::from_secs(60));
        // Test cache hit/miss scenarios
    }

    #[tokio::test]
    async fn test_streaming_detection_failure() {
        // Test detection when endpoint returns error
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        // Test that cache expires correctly
    }
}
```

**File**: `proxy/src/fallback_coordinator.rs` (extend with tests)

**Fallback Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_activation() {
        // Test fallback when streaming fails
    }

    #[tokio::test]
    async fn test_fallback_disabled() {
        // Test behavior when fallback is disabled
    }

    #[tokio::test]
    async fn test_statistics_tracking() {
        // Test that statistics are tracked correctly
    }
}
```

### 2. Integration Tests for Complete Streaming Flow
**File**: `proxy/tests/streaming_integration_tests.rs` (new file)

**End-to-End Tests**:
```rust
use actix_web::{test, web, App};
use serde_json::json;

#[actix_web::test]
async fn test_streaming_when_supported() {
    // Mock Straico to support streaming
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(create_test_app_state()))
            .service(openai_chat_completion)
    ).await;

    let req = test::TestRequest::post()
        .uri("/v1/chat/completions")
        .set_json(&json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": true
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/event-stream");
}

#[actix_web::test]
async fn test_fallback_when_streaming_unsupported() {
    // Mock Straico to not support streaming
    // Verify fallback to non-streaming with SSE wrapper
}

#[actix_web::test]
async fn test_streaming_with_tools() {
    // Test streaming with tool calls embedded
}

#[actix_web::test]
async fn test_streaming_error_handling() {
    // Test various error scenarios during streaming
}
```

### 3. Mock Straico Server for Testing
**File**: `proxy/tests/mock_straico_server.rs` (new file)

**Mock Server Implementation**:
```rust
use actix_web::{web, App, HttpResponse, HttpServer, Result};
use serde_json::json;

pub struct MockStraicoServer {
    pub supports_streaming: bool,
    pub should_fail: bool,
    pub response_delay: Duration,
}

impl MockStraicoServer {
    pub async fn start(config: MockStraicoConfig) -> String {
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(config.clone()))
                .route("/v0/chat/completions", web::post().to(mock_chat_completions))
        })
        .bind("127.0.0.1:0")
        .unwrap()
        .run();

        let addr = server.addrs()[0];
        tokio::spawn(server);
        format!("http://127.0.0.1:{}", addr.port())
    }
}

async fn mock_chat_completions(
    req: web::Json<serde_json::Value>,
    config: web::Data<MockStraicoConfig>,
) -> Result<HttpResponse> {
    if config.should_fail {
        return Ok(HttpResponse::InternalServerError().json(json!({
            "error": "Mock server error"
        })));
    }

    if config.supports_streaming && req.get("stream").is_some() {
        // Return streaming response
        let stream = create_mock_sse_stream();
        Ok(HttpResponse::Ok()
            .content_type("text/event-stream")
            .streaming(stream))
    } else {
        // Return non-streaming response
        tokio::time::sleep(config.response_delay).await;
        Ok(HttpResponse::Ok().json(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Mock response"
                },
                "finish_reason": "stop"
            }],
            "model": "mock-model"
        })))
    }
}
```

### 4. Performance Tests for Streaming
**File**: `proxy/tests/streaming_performance_tests.rs` (new file)

**Performance Benchmarks**:
```rust
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

fn benchmark_streaming_detection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("streaming_detection", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Benchmark streaming detection performance
            })
        })
    });
}

fn benchmark_fallback_activation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("fallback_activation", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Benchmark fallback performance
            })
        })
    });
}

fn benchmark_stream_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("stream_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Benchmark stream chunk processing
            })
        })
    });
}

criterion_group!(
    benches,
    benchmark_streaming_detection,
    benchmark_fallback_activation,
    benchmark_stream_processing
);
criterion_main!(benches);
```

### 5. Manual Testing Scripts
**File**: `scripts/test_streaming_scenarios.sh` (new file)

**Comprehensive Manual Tests**:
```bash
#!/bin/bash

BASE_URL="http://localhost:8000"
API_KEY="test-key"

echo "=== Testing Streaming Scenarios ==="

echo "1. Testing basic streaming..."
curl -N -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "model": "test-model",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'

echo -e "\n\n2. Testing streaming with tools..."
curl -N -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "model": "test-model",
    "messages": [{"role": "user", "content": "What is the weather?"}],
    "stream": true,
    "tools": [{
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get current weather"
      }
    }]
  }'

echo -e "\n\n3. Testing fallback scenario..."
# Test with model that doesn't support streaming
curl -N -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "model": "non-streaming-model",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'

echo -e "\n\n4. Testing error handling..."
# Test with invalid request
curl -N -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "model": "",
    "messages": [],
    "stream": true
  }'

echo -e "\n\n5. Testing health endpoint..."
curl -X GET "$BASE_URL/health/streaming" \
  -H "Authorization: Bearer $API_KEY"
```

### 6. Load Testing for Streaming
**File**: `scripts/load_test_streaming.sh` (new file)

**Load Test Script**:
```bash
#!/bin/bash

echo "=== Streaming Load Test ==="

# Test concurrent streaming requests
for i in {1..10}; do
  (
    echo "Starting stream $i..."
    curl -N -X POST "http://localhost:8000/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -d '{
        "model": "test-model",
        "messages": [{"role": "user", "content": "Stream test '$i'"}],
        "stream": true
      }' &
  )
done

wait
echo "All streams completed"
```

### 7. Error Scenario Testing
**File**: `proxy/tests/streaming_error_tests.rs` (new file)

**Error Scenario Tests**:
```rust
#[tokio::test]
async fn test_stream_timeout() {
    // Test streaming timeout handling
}

#[tokio::test]
async fn test_connection_failure() {
    // Test handling of connection failures
}

#[tokio::test]
async fn test_malformed_chunks() {
    // Test handling of malformed SSE chunks
}

#[tokio::test]
async fn test_fallback_failure() {
    // Test when both streaming and fallback fail
}

#[tokio::test]
async fn test_detection_cache_failure() {
    // Test when detection cache fails
}
```

### 8. Health Monitoring Tests
**File**: `proxy/tests/health_monitoring_tests.rs` (new file)

**Health Tests**:
```rust
#[tokio::test]
async fn test_health_monitoring() {
    let monitor = StreamingHealthMonitor::new();
    
    // Simulate successful streams
    monitor.record_stream_start().await;
    monitor.record_stream_success(Duration::from_secs(1)).await;
    
    assert!(monitor.is_healthy().await);
    
    // Simulate failures
    for _ in 0..6 {
        monitor.record_stream_failure().await;
    }
    
    assert!(!monitor.is_healthy().await);
}

#[tokio::test]
async fn test_health_report_generation() {
    // Test health report accuracy
}
```

## Deliverables

1. **Comprehensive Test Suite**:
   - Unit tests for all streaming components
   - Integration tests for complete flows
   - Performance benchmarks
   - Error scenario tests

2. **Testing Infrastructure**:
   - Mock Straico server for testing
   - Manual testing scripts
   - Load testing utilities
   - Health monitoring tests

3. **Test Documentation**:
   - Test coverage report
   - Performance baseline metrics
   - Known limitations and edge cases
   - Testing procedures documentation

## Success Criteria

- [ ] All unit tests pass (>90% coverage)
- [ ] Integration tests cover all streaming scenarios
- [ ] Performance tests show acceptable metrics
- [ ] Error handling tests pass all scenarios
- [ ] Manual testing scripts work correctly
- [ ] Load tests demonstrate stability
- [ ] Health monitoring functions properly
- [ ] Mock server enables comprehensive testing

## Time Estimate
**Duration**: 6-8 hours

## Dependencies
- **P3-T5**: Update Error Handling and Fallbacks

## Next Task
None (Phase 3 complete)

## Notes
- Focus on edge cases and failure scenarios
- Ensure tests can run in CI/CD pipeline
- Document any discovered limitations
- Establish performance baselines for monitoring
- Test with various network conditions and latencies
- Verify OpenAI compatibility in all scenarios