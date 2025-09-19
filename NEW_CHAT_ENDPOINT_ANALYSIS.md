# New Chat Endpoint Migration Analysis

## Executive Summary

This report analyzes the changes required to migrate from the current Straico prompt endpoint (`/v1/prompt/completion`) to the new chat endpoint (`/v0/chat/completions`). The migration involves significant structural changes to both request and response formats, with particular challenges around tool calls and streaming functionality.

## Current Architecture Overview

### Current Endpoint
- **URL**: `https://api.straico.com/v1/prompt/completion`
- **Format**: Uses a prompt-based approach where chat messages are converted to formatted text strings
- **Streaming**: Fully implemented with heartbeat chunks and proper SSE formatting
- **Tool Calls**: Supported via XML-style markup in prompt text

### New Endpoint
- **URL**: `https://api.straico.com/v0/chat/completions`
- **Format**: Native chat format with structured message arrays
- **Streaming**: Not yet supported by Straico API
- **Tool Calls**: Not yet supported by Straico API

## Detailed Analysis

### 1. Request Format Changes

#### Current Format (Prompt-based)
```rust
struct CompletionRequest {
    models: RequestModels,
    message: Prompt,  // Single formatted string
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    // ... other fields
}
```

#### New Format (Chat-based)
```json
{
    "model": "meta-llama/llama-4-maverick",
    "messages": [
        {
            "role": "user",
            "content": [{"type": "text", "text": "solve for x => 10x -4x = 0"}]
        }
    ],
    "temperature": 0.7,
    "max_tokens": 150
}
```

**Key Differences:**
1. **Single model vs multiple models**: New endpoint accepts only one model string
2. **Message structure**: Content is now an array of objects instead of plain text
3. **No prompt conversion**: Messages stay as structured data instead of being flattened to text

### 2. Response Format Changes

The current response parsing logic expects the old format. The new endpoint will likely return responses in a different structure that needs investigation.

### 3. Critical Issues Identified

#### 🔴 **CRITICAL: Tool Calls Not Supported**
- **Current**: Tool calls work via XML markup in prompt text (`<tool_call>...</tool_call>`)
- **New**: Tool calls are explicitly listed as "Non-available parameters"
- **Impact**: Complete loss of tool functionality until Straico implements it
- **Risk**: High - This is a major feature regression

#### 🔴 **CRITICAL: Streaming Not Supported**
- **Current**: Full streaming implementation with heartbeat chunks
- **New**: Streaming is explicitly listed as "Non-available parameters"
- **Impact**: All streaming requests will need to fall back to non-streaming
- **Risk**: High - Performance and UX degradation

#### 🟡 **MODERATE: Content Structure Change**
- **Current**: `content: "simple string"`
- **New**: `content: [{"type": "text", "text": "string"}]`
- **Impact**: Need to restructure all message handling
- **Risk**: Medium - Requires careful migration of message parsing

#### 🟡 **MODERATE: Model Selection Change**
- **Current**: Supports multiple models in single request
- **New**: Single model per request
- **Impact**: Need to change model selection logic
- **Risk**: Medium - May affect load balancing strategies

#### 🟢 **LOW: URL Change**
- **Current**: `/v1/prompt/completion`
- **New**: `/v0/chat/completions`
- **Impact**: Simple URL update in client
- **Risk**: Low - Easy to change

### 4. Code Areas Requiring Changes

#### 4.1 Client Library (`client/`)

**Files to modify:**
- `src/client.rs`: Update endpoint URL and request builder
- `src/endpoints/completion/completion_request.rs`: Create new chat request structure
- `src/endpoints/completion/completion_response.rs`: Handle new response format
- `src/chat.rs`: Modify message handling for new content structure

**New structures needed:**
```rust
#[derive(Serialize)]
struct ChatRequest {
    model: String,  // Single model instead of multiple
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: Vec<ContentObject>,
}

#[derive(Serialize)]
struct ContentObject {
    #[serde(rename = "type")]
    content_type: String,  // Always "text" for now
    text: String,
}
```

#### 4.2 Proxy Server (`proxy/`)

**Files to modify:**
- `src/server.rs`: Update request/response conversion logic
- `src/streaming.rs`: Handle streaming fallback when not supported

**Key changes:**
1. **Request conversion**: Convert OpenAI format to new Straico chat format
2. **Response handling**: Parse new response structure
3. **Streaming fallback**: Detect when streaming is not available and fall back gracefully

### 5. Migration Strategy

#### Phase 1: Basic Chat Support (No Tools, No Streaming)
1. Create new chat request/response structures
2. Update client to use new endpoint
3. Modify proxy to convert between OpenAI and new Straico format
4. Disable tool calls and streaming temporarily

#### Phase 2: Streaming Fallback
1. Implement detection for streaming support
2. Fall back to non-streaming when not available
3. Add configuration option to force non-streaming mode

#### Phase 3: Tool Calls (When Available)
1. Wait for Straico to implement tool calls in new endpoint
2. Update structures to support new tool call format
3. Migrate tool call conversion logic

### 6. Compatibility Concerns

#### Backward Compatibility
- **Risk**: High risk of breaking existing integrations
- **Mitigation**: Consider maintaining both endpoints during transition period
- **Timeline**: Coordinate with Straico API deprecation schedule

#### Feature Parity
- **Current State**: New endpoint has fewer features than current
- **Timeline**: Unknown when tool calls and streaming will be available
- **Risk**: Users may experience feature regression

### 7. Testing Strategy

#### Unit Tests
- Test message format conversion
- Test model selection logic
- Test error handling for unsupported features

#### Integration Tests
- Test against actual new Straico endpoint
- Verify OpenAI compatibility is maintained
- Test fallback mechanisms

#### Performance Tests
- Compare streaming vs non-streaming performance
- Test with various message sizes and complexities

### 8. Recommendations

#### Immediate Actions
1. **Create feature flags** to switch between old and new endpoints
2. **Implement new chat structures** alongside existing ones
3. **Add comprehensive logging** to track migration issues

#### Risk Mitigation
1. **Gradual rollout** with ability to rollback quickly
2. **Monitor Straico roadmap** for tool calls and streaming support
3. **Consider hybrid approach** using old endpoint for advanced features

#### Timeline Considerations
1. **Don't migrate until streaming is supported** (if streaming is critical)
2. **Wait for tool calls support** if tools are essential
3. **Consider partial migration** for simple chat-only use cases

## Conclusion

The migration to the new chat endpoint represents a significant architectural change with both benefits and risks. While the new format is more structured and aligns better with modern chat APIs, the temporary loss of streaming and tool calls creates substantial compatibility concerns.

**Recommendation**: Implement the new endpoint support alongside the existing one, allowing users to choose based on their feature requirements. Monitor Straico's development of the missing features before making it the default.