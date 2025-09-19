# P2-T5: Update OpenAI Compatibility Layer

## Objective
Update the proxy server's OpenAI compatibility layer to seamlessly handle tool calls with the new chat endpoint while maintaining full API compatibility.

## Background
The proxy needs to present a unified OpenAI-compatible interface regardless of which Straico endpoint is being used. This task ensures tool calling works identically from the client's perspective.

## Tasks

### 1. Update OpenAI Request Handler
**File**: `proxy/src/server.rs`

**Enhanced Chat Endpoint Handler**:
```rust
#[post("/v1/chat/completions")]
async fn openai_chat_completion(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<Either<web::Json<Completion>, HttpResponse>, CustomError> {
    let req_inner = req.into_inner();
    
    if data.print_request_raw {
        debug!("\n\n===== Request received (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&req_inner).unwrap());
    }

    let openai_request: OpenAiChatRequest = serde_json::from_value(req_inner.clone())?;

    // Choose endpoint based on configuration
    if data.use_new_chat_endpoint {
        handle_new_chat_endpoint(openai_request, data).await
    } else {
        handle_legacy_completion_endpoint(openai_request, data).await
    }
}

async fn handle_new_chat_endpoint(
    openai_request: OpenAiChatRequest,
    data: web::Data<AppState>,
) -> Result<Either<web::Json<Completion>, HttpResponse>, CustomError> {
    // Convert to new chat format with tool embedding
    let chat_request = if openai_request.tools.is_some() {
        tool_embedding::embed_tools_in_chat_request(openai_request)?
    } else {
        content_conversion::convert_openai_to_chat_request(openai_request)
    };

    if data.print_request_converted {
        debug!("\n\n===== Request converted (new chat): =====");
        debug!("\n{}", serde_json::to_string_pretty(&chat_request).unwrap());
    }

    // Send to new chat endpoint
    let client = data.client.clone();
    let chat_response = client
        .chat()
        .bearer_auth(&data.key)
        .json(chat_request)
        .send()
        .await?
        .get_chat_completion()?;

    // Parse tool calls and convert to OpenAI format
    let parsed_response = chat_response.parse_tool_calls(&chat_request.model)?;
    let completion_response = convert_chat_response_to_completion(
        parsed_response,
        &generate_request_id(),
        current_timestamp(),
    );

    if data.print_response_converted {
        debug!("\n\n===== Response converted: =====");
        debug!("\n{}", serde_json::to_string_pretty(&completion_response).unwrap());
    }

    Ok(Either::Left(web::Json(completion_response)))
}

async fn handle_legacy_completion_endpoint(
    openai_request: OpenAiChatRequest,
    data: web::Data<AppState>,
) -> Result<Either<web::Json<Completion>, HttpResponse>, CustomError> {
    // Convert to legacy OpenAiRequest format
    let legacy_request = convert_chat_to_legacy_request(openai_request);
    
    // Use existing completion logic
    // ... existing implementation
}
```

### 2. Create Request Format Conversion
**File**: `proxy/src/request_conversion.rs` (new file)

**Bidirectional Conversion**:
```rust
use crate::openai_types::{OpenAiChatRequest, OpenAiChatMessage, OpenAiContent};
use straico_client::chat::{Chat, Tool};

/// Convert new chat request to legacy completion request format
pub fn convert_chat_to_legacy_request(chat_request: OpenAiChatRequest) -> OpenAiRequest {
    // Convert messages to Chat format for legacy endpoint
    let messages: Vec<Message> = chat_request.messages
        .into_iter()
        .map(|msg| convert_openai_message_to_legacy(msg))
        .collect();

    OpenAiRequest {
        model: chat_request.model.into(),
        messages: Chat(messages),
        max_tokens: chat_request.max_tokens,
        temperature: chat_request.temperature,
        stream: false, // Handle streaming separately
        tools: chat_request.tools,
    }
}

fn convert_openai_message_to_legacy(msg: OpenAiChatMessage) -> Message {
    let content_text = match msg.content {
        OpenAiContent::String(text) => text,
        OpenAiContent::Array(objects) => {
            objects.into_iter()
                .filter(|obj| obj.content_type == "text")
                .map(|obj| obj.text)
                .collect::<Vec<_>>()
                .join(" ")
        }
    };

    match msg.role.as_str() {
        "user" => Message::User { 
            content: Content::Text(content_text.into()) 
        },
        "assistant" => Message::Assistant { 
            content: Some(Content::Text(content_text.into())),
            tool_calls: None, // Will be parsed from content
        },
        "system" => Message::System { 
            content: Content::Text(content_text.into()) 
        },
        _ => Message::User { 
            content: Content::Text(content_text.into()) 
        },
    }
}
```

### 3. Add Configuration Management
**File**: `proxy/src/main.rs`

**Configuration Options**:
```rust
#[derive(Parser)]
struct Cli {
    // ... existing fields
    
    /// Use new chat endpoint instead of legacy completion endpoint
    #[arg(long)]
    use_new_chat_endpoint: bool,
    
    /// Force tool calls to use new endpoint (even if legacy is default)
    #[arg(long)]
    force_new_endpoint_for_tools: bool,
}

#[derive(Clone)]
struct AppState {
    // ... existing fields
    use_new_chat_endpoint: bool,
    force_new_endpoint_for_tools: bool,
}
```

### 4. Implement Endpoint Selection Logic
**File**: `proxy/src/endpoint_selection.rs` (new file)

**Smart Endpoint Selection**:
```rust
pub fn should_use_new_endpoint(
    request: &OpenAiChatRequest,
    config: &AppState,
) -> bool {
    // Always use new endpoint if configured
    if config.use_new_chat_endpoint {
        return true;
    }
    
    // Use new endpoint for tool calls if forced
    if config.force_new_endpoint_for_tools && request.tools.is_some() {
        return true;
    }
    
    // Default to legacy endpoint
    false
}

pub fn validate_request_for_endpoint(
    request: &OpenAiChatRequest,
    use_new_endpoint: bool,
) -> Result<(), String> {
    if use_new_endpoint {
        // Validate new endpoint requirements
        if request.messages.is_empty() {
            return Err("Messages cannot be empty for chat endpoint".to_string());
        }
        
        // Check for unsupported features
        // (streaming will be added in Phase 3)
    }
    
    Ok(())
}
```

### 5. Update Error Handling
**File**: `proxy/src/error.rs`

**Enhanced Error Types**:
```rust
#[derive(Error, Debug)]
pub enum CustomError {
    // ... existing variants
    
    #[error("Chat endpoint error: {0}")]
    ChatEndpoint(String),
    
    #[error("Request validation error: {0}")]
    RequestValidation(String),
    
    #[error("Endpoint selection error: {0}")]
    EndpointSelection(String),
}

impl CustomError {
    pub fn to_openai_error_response(&self) -> HttpResponse {
        let error_message = match self {
            CustomError::ToolEmbedding(e) => format!("Tool processing failed: {}", e),
            CustomError::ChatEndpoint(e) => format!("Chat endpoint error: {}", e),
            CustomError::RequestValidation(e) => format!("Invalid request: {}", e),
            _ => self.to_string(),
        };

        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": {
                "message": error_message,
                "type": "invalid_request_error",
                "code": null
            }
        }))
    }
}
```

### 6. Add Integration Tests
**File**: `proxy/tests/openai_compatibility_tests.rs`

**Comprehensive Test Suite**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_calls_new_endpoint() {
        let request = create_openai_request_with_tools();
        let response = send_chat_completion_request(request).await;
        
        assert!(response.choices[0].message.tool_calls.is_some());
        assert_eq!(response.choices[0].finish_reason, "tool_calls");
    }
    
    #[tokio::test]
    async fn test_tool_calls_legacy_endpoint() {
        let request = create_openai_request_with_tools();
        let response = send_completion_request(request).await;
        
        // Should produce identical results
        assert!(response.choices[0].message.tool_calls.is_some());
    }
    
    #[tokio::test]
    async fn test_endpoint_selection() {
        // Test various configuration scenarios
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        // Test error responses match OpenAI format
    }
}
```

## Deliverables

1. **Enhanced Server Handler**:
   - Updated `/v1/chat/completions` endpoint
   - Dual endpoint support
   - Smart endpoint selection

2. **Conversion Utilities**:
   - `proxy/src/request_conversion.rs`
   - `proxy/src/endpoint_selection.rs`
   - Bidirectional format conversion

3. **Configuration Management**:
   - Command-line flags for endpoint selection
   - Runtime configuration options
   - Validation logic

4. **Error Handling**:
   - OpenAI-compatible error responses
   - Comprehensive error types
   - Proper HTTP status codes

## Success Criteria

- [ ] OpenAI API compatibility maintained
- [ ] Tool calls work identically on both endpoints
- [ ] Configuration allows endpoint selection
- [ ] Error responses match OpenAI format
- [ ] Integration tests pass
- [ ] No breaking changes to existing API
- [ ] Performance is acceptable

## Time Estimate
**Duration**: 3-4 hours

## Dependencies
- **P2-T4**: Adapt Tool Response Parsing

## Next Task
**P2-T6**: Testing and Validation

## Notes
- Maintain strict OpenAI API compatibility
- Ensure tool calling behavior is identical between endpoints
- Test thoroughly with real OpenAI client libraries
- Document any configuration options clearly