# P3-T3: Implement Streaming Request Handling

## Objective
Implement the streaming request handling logic for the new chat endpoint, including detection, fallback mechanisms, and request processing.

## Background
Based on the design from P3-T2, implement the actual streaming functionality for the new chat endpoint with proper fallback when streaming is not available.

## Tasks

### 1. Implement Streaming Detection
**File**: `proxy/src/streaming_detection.rs` (new file)

**Core Implementation**:
```rust
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

pub struct StreamingCapabilityCache {
    capabilities: RwLock<HashMap<String, (bool, SystemTime)>>,
    cache_duration: Duration,
}

impl StreamingCapabilityCache {
    pub fn new(cache_duration: Duration) -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
            cache_duration,
        }
    }

    pub async fn check_streaming_support(
        &self,
        client: &StraicoClient,
        model: &str,
    ) -> Result<bool, StreamingDetectionError> {
        // Check cache first
        if let Some(cached) = self.get_cached_capability(model).await {
            return Ok(cached);
        }

        // Perform detection
        let supports_streaming = self.detect_streaming_capability(client, model).await?;
        
        // Cache result
        self.cache_capability(model, supports_streaming).await;
        
        Ok(supports_streaming)
    }

    async fn detect_streaming_capability(
        &self,
        client: &StraicoClient,
        model: &str,
    ) -> Result<bool, StreamingDetectionError> {
        // Create minimal test request
        let test_request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: vec![ContentObject {
                    content_type: "text".to_string(),
                    text: "test".to_string(),
                }],
            }],
            temperature: None,
            max_tokens: Some(1),
        };

        // Try streaming request
        match client.chat_streaming()
            .bearer_auth(&"test_key")
            .json(test_request)
            .send()
            .await 
        {
            Ok(_) => Ok(true),
            Err(e) if e.is_streaming_not_supported() => Ok(false),
            Err(e) => Err(StreamingDetectionError::RequestFailed(e.to_string())),
        }
    }
}
```

### 2. Implement Chat Streaming Client Method
**File**: `client/src/client.rs`

**Add Streaming Method**:
```rust
impl StraicoClient {
    /// Creates a request builder for the new chat endpoint with streaming
    pub fn chat_streaming<'a>(self) -> StraicoRequestBuilder<NoApiKey, ChatRequest> {
        self.0
            .post("https://api.straico.com/v0/chat/completions")
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .into()
    }
}

// Add streaming-specific request builder methods
impl StraicoRequestBuilder<ApiKeySet, PayloadSet> {
    /// Sends a streaming request and returns an async stream
    pub async fn send_streaming(self) -> Result<impl Stream<Item = Result<ChatStreamChunk, StraicoError>>, StraicoError> {
        let response = self.0.send().await?;
        
        if !response.status().is_success() {
            return Err(StraicoError::Api(format!("Request failed: {}", response.status())));
        }

        // Check if response is actually streaming
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/event-stream") {
            return Err(StraicoError::Api("Response is not a stream".to_string()));
        }

        Ok(response.bytes_stream().map(|chunk| {
            chunk.map_err(StraicoError::Request)
                .and_then(|bytes| parse_sse_chunk(&bytes))
        }))
    }
}
```

### 3. Implement Streaming Request Handler
**File**: `proxy/src/chat_streaming.rs` (new file)

**Main Streaming Handler**:
```rust
use crate::streaming_detection::StreamingCapabilityCache;
use crate::content_conversion::convert_openai_to_chat_request;

pub struct ChatStreamingHandler {
    capability_cache: StreamingCapabilityCache,
    config: StreamingConfig,
}

impl ChatStreamingHandler {
    pub async fn handle_streaming_request(
        &self,
        openai_request: OpenAiChatRequest,
        client: &StraicoClient,
        api_key: &str,
    ) -> Result<impl Stream<Item = Result<web::Bytes, CustomError>>, CustomError> {
        // Check if streaming is supported
        let supports_streaming = self.capability_cache
            .check_streaming_support(client, &openai_request.model)
            .await?;

        if supports_streaming && !self.config.force_non_streaming {
            self.handle_streaming_chat_request(openai_request, client, api_key).await
        } else {
            self.handle_non_streaming_with_sse_wrapper(openai_request, client, api_key).await
        }
    }

    async fn handle_streaming_chat_request(
        &self,
        openai_request: OpenAiChatRequest,
        client: &StraicoClient,
        api_key: &str,
    ) -> Result<impl Stream<Item = Result<web::Bytes, CustomError>>, CustomError> {
        // Convert to chat request format
        let chat_request = convert_openai_to_chat_request(openai_request.clone())?;

        // Start streaming request
        let stream = client.chat_streaming()
            .bearer_auth(api_key)
            .json(chat_request)
            .send_streaming()
            .await?;

        // Convert stream to SSE format
        Ok(self.convert_chat_stream_to_sse(stream, &openai_request.model))
    }

    async fn handle_non_streaming_with_sse_wrapper(
        &self,
        openai_request: OpenAiChatRequest,
        client: &StraicoClient,
        api_key: &str,
    ) -> Result<impl Stream<Item = Result<web::Bytes, CustomError>>, CustomError> {
        // Make non-streaming request
        let chat_request = convert_openai_to_chat_request(openai_request.clone())?;
        
        let response = client.chat()
            .bearer_auth(api_key)
            .json(chat_request)
            .send()
            .await?;

        // Wrap response in SSE format
        Ok(self.wrap_non_streaming_response_as_sse(response, &openai_request.model))
    }
}
```

### 4. Implement SSE Conversion
**File**: `proxy/src/chat_streaming.rs`

**SSE Conversion Logic**:
```rust
impl ChatStreamingHandler {
    fn convert_chat_stream_to_sse(
        &self,
        stream: impl Stream<Item = Result<ChatStreamChunk, StraicoError>>,
        model: &str,
    ) -> impl Stream<Item = Result<web::Bytes, CustomError>> {
        let stream_id = generate_stream_id();
        let model = model.to_string();

        // Create initial chunk
        let initial_chunk = create_initial_chunk(&model, &stream_id);
        let initial_stream = stream::once(async move {
            Ok(web::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::to_string(&initial_chunk).unwrap()
            )))
        });

        // Process streaming chunks
        let chunk_stream = stream.map(move |chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    // Convert ChatStreamChunk to CompletionStream format
                    let completion_chunk = convert_chat_chunk_to_completion(chunk, &model, &stream_id);
                    let json = serde_json::to_string(&completion_chunk).unwrap();
                    Ok(web::Bytes::from(format!("data: {}\n\n", json)))
                }
                Err(e) => {
                    let error_chunk = create_error_chunk(&e.to_string());
                    let json = serde_json::to_string(&error_chunk).unwrap();
                    Ok(web::Bytes::from(format!("data: {}\n\n", json)))
                }
            }
        });

        // Add termination
        let end_stream = stream::once(async {
            Ok(web::Bytes::from("data: [DONE]\n\n"))
        });

        initial_stream.chain(chunk_stream).chain(end_stream)
    }

    fn wrap_non_streaming_response_as_sse(
        &self,
        response: ChatResponse,
        model: &str,
    ) -> impl Stream<Item = Result<web::Bytes, CustomError>> {
        let stream_id = generate_stream_id();
        
        // Convert response to streaming format
        let completion_stream = CompletionStream::from(response);
        
        // Create SSE stream from completion
        create_streaming_response_from_completion(completion_stream, model.to_string(), stream_id)
    }
}
```

### 5. Implement Configuration
**File**: `proxy/src/streaming_config.rs` (new file)

**Configuration Structure**:
```rust
#[derive(Clone, Debug)]
pub struct StreamingConfig {
    pub force_non_streaming: bool,
    pub detection_cache_duration: Duration,
    pub enable_fallback: bool,
    pub streaming_timeout: Duration,
    pub heartbeat_interval: Duration,
    pub max_retries: u32,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            force_non_streaming: false,
            detection_cache_duration: Duration::from_secs(300), // 5 minutes
            enable_fallback: true,
            streaming_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(15),
            max_retries: 3,
        }
    }
}

impl StreamingConfig {
    pub fn from_env() -> Self {
        Self {
            force_non_streaming: std::env::var("FORCE_NON_STREAMING")
                .map(|v| v.parse().unwrap_or(false))
                .unwrap_or(false),
            // ... other config from env
            ..Default::default()
        }
    }
}
```

### 6. Update Server Integration
**File**: `proxy/src/server.rs`

**Integration with Existing Handler**:
```rust
// Add to AppState
struct AppState {
    client: StraicoClient,
    key: String,
    streaming_handler: ChatStreamingHandler, // Add this
    // ... existing fields
}

// Update openai_completion handler
#[post("/v1/chat/completions")]
async fn openai_completion(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<Either<web::Json<Completion>, HttpResponse>, CustomError> {
    // ... existing logic for parsing request

    if req_inner_oa.stream {
        // Use new streaming handler
        let stream = data.streaming_handler
            .handle_streaming_request(req_inner_oa, &data.client, &data.key)
            .await?;

        Ok(Either::Right(
            HttpResponseBuilder::new(StatusCode::OK)
                .content_type("text/event-stream")
                .append_header(("Cache-Control", "no-cache"))
                .append_header(("Connection", "keep-alive"))
                .streaming(stream),
        ))
    } else {
        // ... existing non-streaming logic
    }
}
```

## Deliverables

1. **New Modules**:
   - `proxy/src/streaming_detection.rs` - Streaming capability detection
   - `proxy/src/chat_streaming.rs` - Main streaming handler
   - `proxy/src/streaming_config.rs` - Configuration management

2. **Client Updates**:
   - New `chat_streaming()` method
   - Streaming request builder methods
   - SSE response handling

3. **Server Integration**:
   - Updated request handler
   - Streaming capability integration
   - Configuration management

## Success Criteria

- [ ] Streaming detection works correctly
- [ ] Streaming requests handled properly
- [ ] Fallback mechanism functions
- [ ] SSE format maintained for compatibility
- [ ] Configuration options work
- [ ] Error handling robust
- [ ] Integration with existing server complete

## Time Estimate
**Duration**: 5-6 hours

## Dependencies
- **P3-T2**: Design Streaming Architecture for New Endpoint

## Next Task
**P3-T4**: Adapt Streaming Response Processing

## Notes
- Focus on robust error handling and fallback
- Ensure SSE format matches existing implementation
- Test streaming detection thoroughly
- Consider performance impact of detection caching