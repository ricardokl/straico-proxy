# P3-T2: Design Streaming Architecture for New Endpoint

## Objective
Design the streaming architecture for the new chat endpoint, including fallback mechanisms and detection of streaming support availability.

## Background
Based on the analysis from P3-T1, design how to adapt the current streaming implementation to work with the new chat endpoint when Straico adds streaming support.

## Tasks

### 1. Design Streaming Detection Mechanism
**File**: `STREAMING_DETECTION_DESIGN.md`

**Detection Strategy**:
- Attempt streaming request to new endpoint
- Detect streaming support from response headers/status
- Cache streaming capability per model
- Fallback to non-streaming when unavailable

**Detection Implementation**:
```rust
pub struct StreamingCapability {
    pub endpoint_supports_streaming: bool,
    pub model_supports_streaming: HashMap<String, bool>,
    pub last_checked: SystemTime,
}

pub async fn detect_streaming_support(
    client: &StraicoClient,
    model: &str
) -> Result<bool, StreamingDetectionError> {
    // Try streaming request with minimal payload
    // Check response for streaming indicators
    // Cache result for future requests
}
```

### 2. Design Streaming Request Adaptation
**File**: `proxy/src/chat_streaming.rs` (new file design)

**Adapted Streaming Flow**:
```
OpenAI streaming request
    ↓
Check streaming capability for model
    ↓
If supported: New chat endpoint with streaming
If not: Fallback to non-streaming
    ↓
Process response chunks (adapted format)
    ↓
Convert to OpenAI SSE format
    ↓
Send to client
```

**Key Adaptations**:
- New endpoint URL for streaming
- Adapted request format (ChatRequest)
- New response chunk processing
- Same SSE output format

### 3. Design Response Chunk Processing
**Chunk Processing Strategy**:
- Reuse existing `CompletionStream` structures if possible
- Adapt for new response format from Straico
- Maintain same SSE output format for OpenAI compatibility

**Adaptation Points**:
```rust
// Existing
impl From<Completion> for CompletionStream

// New - adapt for chat response format
impl From<ChatResponse> for CompletionStream
impl From<ChatChoice> for ChoiceStream
impl From<ChatMessage> for Delta
```

### 4. Design Fallback Architecture
**Fallback Strategy**:
- Graceful degradation when streaming unavailable
- Transparent fallback to non-streaming
- Client notification of fallback (optional)

**Fallback Implementation**:
```rust
pub async fn handle_streaming_request(
    request: OpenAiChatRequest,
    client: &StraicoClient
) -> Result<StreamingResponse, StreamingError> {
    if detect_streaming_support(&client, &request.model).await? {
        handle_streaming_chat_request(request, client).await
    } else {
        handle_non_streaming_with_sse_wrapper(request, client).await
    }
}
```

### 5. Design Configuration Options
**Configuration Parameters**:
- Force streaming on/off
- Streaming detection cache duration
- Fallback behavior preferences
- Streaming timeout settings

**Configuration Structure**:
```rust
#[derive(Clone)]
pub struct StreamingConfig {
    pub force_streaming: Option<bool>,
    pub detection_cache_duration: Duration,
    pub enable_fallback: bool,
    pub streaming_timeout: Duration,
    pub heartbeat_interval: Duration,
}
```

### 6. Design Error Handling
**Error Scenarios**:
- Streaming detection failures
- Mid-stream errors from new endpoint
- Fallback activation errors
- SSE formatting errors

**Error Handling Strategy**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum StreamingError {
    #[error("Streaming detection failed: {0}")]
    DetectionFailed(String),
    #[error("Streaming not supported for model: {0}")]
    NotSupported(String),
    #[error("Stream interrupted: {0}")]
    StreamInterrupted(String),
    #[error("Fallback failed: {0}")]
    FallbackFailed(String),
}
```

### 7. Design Testing Strategy
**Test Scenarios**:
- Streaming when supported
- Fallback when not supported
- Detection mechanism accuracy
- Error handling in streams
- Performance comparison

**Mock Testing Approach**:
- Mock Straico responses for testing
- Simulate streaming support detection
- Test fallback scenarios
- Validate SSE output format

## Deliverables

1. **Design Document**:
   - `STREAMING_DETECTION_DESIGN.md` - Complete detection strategy
   - Streaming architecture specification

2. **Module Design**:
   - `proxy/src/chat_streaming.rs` - Streaming logic for new endpoint
   - Configuration structure design
   - Error handling strategy

3. **Integration Plan**:
   - How streaming integrates with chat endpoint
   - Fallback mechanism implementation
   - Configuration management

## Success Criteria

- [ ] Streaming detection mechanism designed
- [ ] Streaming adaptation strategy complete
- [ ] Fallback architecture defined
- [ ] Error handling strategy comprehensive
- [ ] Configuration options specified
- [ ] Testing approach planned
- [ ] Integration points identified

## Time Estimate
**Duration**: 3-4 hours

## Dependencies
- **P3-T1**: Analyze Current Streaming Implementation

## Next Task
**P3-T3**: Implement Streaming Request Handling

## Notes
- Design for graceful degradation when streaming unavailable
- Maintain OpenAI compatibility in SSE output
- Consider caching streaming capability to avoid repeated detection
- Plan for future when Straico fully supports streaming