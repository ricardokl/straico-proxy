# P1-T5: Update Proxy Server for New Endpoint

## Objective
Add a new endpoint handler to the proxy server that converts OpenAI chat requests to the new Straico chat format, maintaining full OpenAI API compatibility.

## Background
The proxy needs to handle OpenAI-style requests at `/v1/chat/completions` and route them to either the old or new Straico endpoint based on configuration.

## Tasks

### 1. Create New Endpoint Handler
**File**: `proxy/src/server.rs`

**Add New Handler Function**:
```rust
/// Handles OpenAI-style chat completion requests using new Straico chat endpoint
#[post("/v1/chat/completions")]
async fn openai_chat_completion(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<web::Json<ChatResponse>, CustomError> {
    let req_inner = req.into_inner();
    
    if data.print_request_raw {
        debug!("\n\n===== Chat Request received (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&req_inner).unwrap());
    }

    // Parse OpenAI request
    let openai_req: OpenAiChatRequest = serde_json::from_value(req_inner.clone())?;
    
    // Validate content formats
    for message in &openai_req.messages {
        validate_openai_content(&message.content)?;
    }

    // Convert to Straico chat format
    let straico_req = convert_openai_to_chat_request(openai_req.clone());
    
    if data.print_request_converted {
        debug!("\n\n===== Chat Request converted: =====");
        debug!("\n{}", serde_json::to_string_pretty(&straico_req).unwrap());
    }

    // Send to new Straico chat endpoint
    let client = data.client.clone();
    let response = client
        .chat_completions()
        .bearer_auth(&data.key)
        .json(straico_req)
        .send_chat()
        .await?
        .get_chat_response()?;

    if data.print_response_raw {
        debug!("\n\n===== Chat Response received (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&response).unwrap());
    }

    // Convert response back to OpenAI format
    let openai_response = convert_chat_response_to_openai(response);
    
    if data.print_response_converted {
        debug!("\n\n===== Chat Response converted: =====");
        debug!("\n{}", serde_json::to_string_pretty(&openai_response).unwrap());
    }

    Ok(web::Json(openai_response))
}
```

### 2. Add Response Conversion
**File**: `proxy/src/content_conversion.rs`

**Response Conversion Functions**:
```rust
use straico_client::endpoints::chat::ChatResponse;

#[derive(Serialize, Debug)]
pub struct OpenAiChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAiChoice>,
    pub usage: Option<OpenAiUsage>,
}

#[derive(Serialize, Debug)]
pub struct OpenAiChoice {
    pub index: u32,
    pub message: OpenAiResponseMessage,
    pub finish_reason: String,
}

#[derive(Serialize, Debug)]
pub struct OpenAiResponseMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Debug)]
pub struct OpenAiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub fn convert_chat_response_to_openai(response: ChatResponse) -> OpenAiChatResponse {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    OpenAiChatResponse {
        id: format!("chatcmpl-{}", generate_random_id()),
        object: "chat.completion".to_string(),
        created,
        model: response.model,
        choices: response.choices.into_iter().enumerate().map(|(i, choice)| {
            OpenAiChoice {
                index: i as u32,
                message: OpenAiResponseMessage {
                    role: choice.message.role,
                    content: choice.message.content,
                },
                finish_reason: choice.finish_reason,
            }
        }).collect(),
        usage: response.usage.map(|u| OpenAiUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
    }
}

fn generate_random_id() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}
```

### 3. Update App Configuration
**File**: `proxy/src/main.rs`

**Add Endpoint Selection**:
```rust
#[derive(Parser)]
struct Cli {
    // ... existing fields ...
    
    /// Use new chat endpoint instead of completion endpoint
    #[arg(long)]
    use_chat_endpoint: bool,
    
    /// Force old completion endpoint (overrides use_chat_endpoint)
    #[arg(long)]
    force_completion_endpoint: bool,
}

#[derive(Clone)]
struct AppState {
    // ... existing fields ...
    
    use_chat_endpoint: bool,
    force_completion_endpoint: bool,
}
```

### 4. Update Service Registration
**File**: `proxy/src/main.rs`

**Register New Service**:
```rust
HttpServer::new(move || {
    App::new()
        .app_data(web::Data::new(AppState {
            client: StraicoClient::new(),
            key: api_key.clone(),
            print_request_raw: cli.print_request_raw,
            print_request_converted: cli.print_request_converted,
            print_response_raw: cli.print_response_raw,
            print_response_converted: cli.print_response_converted,
            use_chat_endpoint: cli.use_chat_endpoint,
            force_completion_endpoint: cli.force_completion_endpoint,
        }))
        .service(server::openai_completion)        // Existing endpoint
        .service(server::openai_chat_completion)   // New endpoint
        .default_service(web::to(HttpResponse::NotFound))
})
```

### 5. Add Endpoint Routing Logic
**File**: `proxy/src/server.rs`

**Smart Routing** (optional enhancement):
```rust
/// Route requests to appropriate endpoint based on configuration
async fn route_chat_request(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<impl Responder, CustomError> {
    if data.force_completion_endpoint {
        // Route to old completion endpoint
        openai_completion(req, data).await
    } else if data.use_chat_endpoint {
        // Route to new chat endpoint
        openai_chat_completion(req, data).await
    } else {
        // Default to completion endpoint for now
        openai_completion(req, data).await
    }
}
```

### 6. Add Error Handling
**File**: `proxy/src/error.rs`

**Chat-Specific Errors**:
```rust
#[derive(Error, Debug)]
pub enum CustomError {
    // ... existing variants ...
    
    #[error("Invalid content format: {0}")]
    InvalidContentFormat(String),
    
    #[error("Chat endpoint error: {0}")]
    ChatEndpoint(String),
}
```

### 7. Update Imports
**File**: `proxy/src/server.rs`

**Add Required Imports**:
```rust
use crate::content_conversion::{
    OpenAiChatRequest, convert_openai_to_chat_request, 
    convert_chat_response_to_openai, validate_openai_content
};
use straico_client::endpoints::chat::ChatResponse;
```

## Deliverables

1. **New Endpoint Handler**:
   - `openai_chat_completion()` function
   - Request parsing and validation
   - Response conversion

2. **Configuration Options**:
   - Command-line flags for endpoint selection
   - App state management
   - Service registration

3. **Response Conversion**:
   - Straico to OpenAI response conversion
   - Proper ID generation and timestamps
   - Usage statistics handling

4. **Error Handling**:
   - Chat-specific error types
   - Validation error handling
   - Graceful failure modes

## Success Criteria

- [ ] New `/v1/chat/completions` endpoint responds
- [ ] OpenAI request format accepted and parsed
- [ ] Content format conversion works correctly
- [ ] Straico chat API called successfully
- [ ] Response converted back to OpenAI format
- [ ] Configuration flags control endpoint selection
- [ ] Error handling works for invalid requests
- [ ] Existing completion endpoint unchanged

## Time Estimate
**Duration**: 3-4 hours

## Dependencies
- **P1-T3**: Implement Content Format Conversion
- **P1-T4**: Add New Endpoint to Client

## Next Task
**P1-T6**: Add Configuration and Feature Flags

## Notes
- Keep existing endpoint fully functional
- Ensure OpenAI compatibility is maintained
- Add comprehensive error handling
- Consider adding request/response logging
- Test with various content formats