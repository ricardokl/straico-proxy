# P3-T5: Update Error Handling and Fallbacks

## Objective
Implement comprehensive error handling and fallback mechanisms for streaming scenarios, ensuring robust operation when streaming fails or is unavailable.

## Background
Streaming introduces additional failure modes that need proper error handling. This task focuses on graceful degradation, error recovery, and user-friendly error responses.

## Tasks

### 1. Define Streaming Error Types
**File**: `proxy/src/streaming_errors.rs` (new file)

**Error Hierarchy**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamingError {
    #[error("Streaming detection failed: {0}")]
    DetectionFailed(String),
    
    #[error("Streaming not supported for model: {0}")]
    NotSupported(String),
    
    #[error("Stream connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Stream interrupted: {0}")]
    StreamInterrupted(String),
    
    #[error("Chunk parsing failed: {0}")]
    ChunkParsingFailed(String),
    
    #[error("Fallback mechanism failed: {0}")]
    FallbackFailed(String),
    
    #[error("Streaming timeout after {timeout}s")]
    Timeout { timeout: u64 },
    
    #[error("Invalid streaming configuration: {0}")]
    InvalidConfig(String),
}

impl StreamingError {
    pub fn to_openai_error(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "message": self.to_string(),
                "type": "streaming_error",
                "code": self.error_code()
            }
        })
    }
    
    fn error_code(&self) -> &'static str {
        match self {
            StreamingError::DetectionFailed(_) => "detection_failed",
            StreamingError::NotSupported(_) => "not_supported",
            StreamingError::ConnectionFailed(_) => "connection_failed",
            StreamingError::StreamInterrupted(_) => "stream_interrupted",
            StreamingError::ChunkParsingFailed(_) => "parsing_failed",
            StreamingError::FallbackFailed(_) => "fallback_failed",
            StreamingError::Timeout { .. } => "timeout",
            StreamingError::InvalidConfig(_) => "invalid_config",
        }
    }
}
```

### 2. Implement Fallback Coordinator
**File**: `proxy/src/fallback_coordinator.rs` (new file)

**Fallback Management**:
```rust
use crate::streaming_errors::StreamingError;
use crate::openai_types::OpenAiChatRequest;

pub struct FallbackCoordinator {
    config: StreamingConfig,
    fallback_stats: Arc<RwLock<FallbackStats>>,
}

#[derive(Default)]
struct FallbackStats {
    total_requests: u64,
    streaming_attempts: u64,
    streaming_successes: u64,
    fallback_activations: u64,
    fallback_failures: u64,
}

impl FallbackCoordinator {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            fallback_stats: Arc::new(RwLock::new(FallbackStats::default())),
        }
    }

    pub async fn execute_with_fallback<F, Fut, T>(
        &self,
        request: &OpenAiChatRequest,
        streaming_fn: F,
        fallback_fn: impl FnOnce() -> Fut,
    ) -> Result<T, StreamingError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, StreamingError>>,
    {
        self.increment_total_requests().await;

        // Try streaming first
        if self.should_attempt_streaming(request).await {
            self.increment_streaming_attempts().await;
            
            match streaming_fn().await {
                Ok(result) => {
                    self.increment_streaming_successes().await;
                    return Ok(result);
                }
                Err(e) => {
                    log::warn!("Streaming failed: {}, attempting fallback", e);
                    
                    if !self.config.enable_fallback {
                        return Err(e);
                    }
                }
            }
        }

        // Execute fallback
        self.increment_fallback_activations().await;
        
        match fallback_fn().await {
            Ok(result) => Ok(result),
            Err(e) => {
                self.increment_fallback_failures().await;
                Err(StreamingError::FallbackFailed(e.to_string()))
            }
        }
    }

    async fn should_attempt_streaming(&self, request: &OpenAiChatRequest) -> bool {
        match self.config.force_streaming {
            Some(true) => true,
            Some(false) => false,
            None => {
                // Check if streaming is likely to work based on recent stats
                let stats = self.fallback_stats.read().await;
                if stats.streaming_attempts == 0 {
                    return true; // First attempt
                }
                
                let success_rate = stats.streaming_successes as f64 / stats.streaming_attempts as f64;
                success_rate > 0.1 // Only attempt if >10% success rate
            }
        }
    }

    pub async fn get_stats(&self) -> FallbackStats {
        self.fallback_stats.read().await.clone()
    }
}
```

### 3. Implement Stream Recovery Mechanisms
**File**: `proxy/src/stream_recovery.rs` (new file)

**Recovery Strategies**:
```rust
use futures::stream::{Stream, StreamExt};
use tokio::time::{timeout, Duration};

pub struct StreamRecovery {
    max_retries: usize,
    retry_delay: Duration,
    timeout_duration: Duration,
}

impl StreamRecovery {
    pub fn new(config: &StreamingConfig) -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
            timeout_duration: config.streaming_timeout,
        }
    }

    pub fn wrap_stream<S, T, E>(
        &self,
        stream: S,
    ) -> impl Stream<Item = Result<T, StreamingError>>
    where
        S: Stream<Item = Result<T, E>> + Unpin,
        E: std::error::Error + Send + Sync + 'static,
        T: Clone,
    {
        let timeout_duration = self.timeout_duration;
        
        stream
            .map(move |item| {
                item.map_err(|e| StreamingError::StreamInterrupted(e.to_string()))
            })
            .timeout(timeout_duration)
            .map(|result| match result {
                Ok(item) => item,
                Err(_) => Err(StreamingError::Timeout {
                    timeout: timeout_duration.as_secs(),
                }),
            })
            .take_while(|item| {
                // Stop stream on certain errors
                match item {
                    Err(StreamingError::ConnectionFailed(_)) => false,
                    Err(StreamingError::FallbackFailed(_)) => false,
                    _ => true,
                }
            })
    }

    pub async fn retry_with_backoff<F, Fut, T>(
        &self,
        mut operation: F,
    ) -> Result<T, StreamingError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, StreamingError>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.max_retries {
                        let delay = self.retry_delay * (2_u32.pow(attempt as u32));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}
```

### 4. Implement Error Response Formatting
**File**: `proxy/src/error_formatting.rs` (new file)

**OpenAI-Compatible Error Responses**:
```rust
use actix_web::{HttpResponse, ResponseError};
use serde_json::json;

impl ResponseError for StreamingError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        
        match self {
            StreamingError::NotSupported(_) => StatusCode::BAD_REQUEST,
            StreamingError::InvalidConfig(_) => StatusCode::BAD_REQUEST,
            StreamingError::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
            StreamingError::ConnectionFailed(_) => StatusCode::BAD_GATEWAY,
            StreamingError::DetectionFailed(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self.to_openai_error())
    }
}

pub fn create_streaming_error_chunk(error: &StreamingError) -> serde_json::Value {
    json!({
        "choices": [{
            "delta": {},
            "index": 0,
            "finish_reason": "error"
        }],
        "object": "chat.completion.chunk",
        "error": error.to_openai_error()
    })
}

pub fn create_fallback_notification_chunk(reason: &str) -> serde_json::Value {
    json!({
        "choices": [{
            "delta": {
                "content": "",
                "role": "assistant"
            },
            "index": 0
        }],
        "object": "chat.completion.chunk",
        "system_message": format!("Fallback activated: {}", reason)
    })
}
```

### 5. Implement Health Monitoring
**File**: `proxy/src/streaming_health.rs` (new file)

**Health Monitoring**:
```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

#[derive(Clone)]
pub struct StreamingHealthMonitor {
    metrics: Arc<RwLock<HealthMetrics>>,
}

#[derive(Default)]
struct HealthMetrics {
    last_successful_stream: Option<Instant>,
    consecutive_failures: u32,
    total_streams: u64,
    successful_streams: u64,
    average_stream_duration: Duration,
}

impl StreamingHealthMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HealthMetrics::default())),
        }
    }

    pub async fn record_stream_start(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_streams += 1;
    }

    pub async fn record_stream_success(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.last_successful_stream = Some(Instant::now());
        metrics.consecutive_failures = 0;
        metrics.successful_streams += 1;
        
        // Update average duration (simple moving average)
        if metrics.successful_streams == 1 {
            metrics.average_stream_duration = duration;
        } else {
            let total_duration = metrics.average_stream_duration * (metrics.successful_streams - 1) as u32 + duration;
            metrics.average_stream_duration = total_duration / metrics.successful_streams as u32;
        }
    }

    pub async fn record_stream_failure(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.consecutive_failures += 1;
    }

    pub async fn is_healthy(&self) -> bool {
        let metrics = self.metrics.read().await;
        
        // Consider unhealthy if too many consecutive failures
        if metrics.consecutive_failures > 5 {
            return false;
        }
        
        // Consider unhealthy if no successful streams in last 10 minutes
        if let Some(last_success) = metrics.last_successful_stream {
            if last_success.elapsed() > Duration::from_secs(600) {
                return false;
            }
        }
        
        true
    }

    pub async fn get_health_report(&self) -> HealthReport {
        let metrics = self.metrics.read().await;
        
        HealthReport {
            is_healthy: self.is_healthy().await,
            total_streams: metrics.total_streams,
            successful_streams: metrics.successful_streams,
            success_rate: if metrics.total_streams > 0 {
                metrics.successful_streams as f64 / metrics.total_streams as f64
            } else {
                0.0
            },
            consecutive_failures: metrics.consecutive_failures,
            last_successful_stream: metrics.last_successful_stream,
            average_stream_duration: metrics.average_stream_duration,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct HealthReport {
    pub is_healthy: bool,
    pub total_streams: u64,
    pub successful_streams: u64,
    pub success_rate: f64,
    pub consecutive_failures: u32,
    pub last_successful_stream: Option<Instant>,
    pub average_stream_duration: Duration,
}
```

### 6. Update Main Streaming Handler
**File**: `proxy/src/chat_streaming.rs` (update existing)

**Add Error Handling Integration**:
```rust
impl ChatStreamingHandler {
    pub async fn handle_streaming_request_with_recovery(
        &self,
        openai_request: OpenAiChatRequest,
        client: &StraicoClient,
        api_key: &str,
    ) -> Result<impl Stream<Item = Result<web::Bytes, CustomError>>, CustomError> {
        let fallback_coordinator = FallbackCoordinator::new(self.config.clone());
        let stream_recovery = StreamRecovery::new(&self.config);
        let health_monitor = StreamingHealthMonitor::new();

        fallback_coordinator.execute_with_fallback(
            &openai_request,
            || async {
                health_monitor.record_stream_start().await;
                let start_time = Instant::now();
                
                let stream = self.handle_true_streaming_internal(
                    openai_request.clone(), 
                    client, 
                    api_key
                ).await?;
                
                let recovered_stream = stream_recovery.wrap_stream(stream);
                
                // Monitor stream completion
                let monitored_stream = recovered_stream.inspect(move |result| {
                    if result.is_err() {
                        let health_monitor = health_monitor.clone();
                        tokio::spawn(async move {
                            health_monitor.record_stream_failure().await;
                        });
                    }
                });
                
                Ok(monitored_stream)
            },
            || async {
                self.handle_non_streaming_with_sse_internal(
                    openai_request.clone(),
                    client,
                    api_key,
                ).await
            },
        ).await
    }
}
```

## Deliverables

1. **Error Handling System**:
   - `proxy/src/streaming_errors.rs` - Comprehensive error types
   - `proxy/src/error_formatting.rs` - OpenAI-compatible error responses

2. **Fallback Mechanisms**:
   - `proxy/src/fallback_coordinator.rs` - Intelligent fallback management
   - `proxy/src/stream_recovery.rs` - Stream recovery and retry logic

3. **Health Monitoring**:
   - `proxy/src/streaming_health.rs` - Health monitoring and metrics

4. **Integration Updates**:
   - Updated streaming handler with error handling
   - Enhanced error responses and logging

## Success Criteria

- [ ] Comprehensive error types defined
- [ ] Fallback mechanisms work correctly
- [ ] Stream recovery handles interruptions
- [ ] Health monitoring tracks streaming status
- [ ] Error responses are OpenAI-compatible
- [ ] Graceful degradation under failure
- [ ] Robust retry mechanisms implemented

## Time Estimate
**Duration**: 4-5 hours

## Dependencies
- **P3-T4**: Adapt Streaming Response Processing

## Next Task
**P3-T6**: Testing and Validation

## Notes
- Focus on graceful degradation and user experience
- Ensure error messages are helpful and actionable
- Monitor performance impact of error handling
- Test failure scenarios thoroughly