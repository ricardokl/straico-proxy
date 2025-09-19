# P1-T4: Add New Endpoint to Client

## Objective
Integrate the new chat endpoint into the Straico client library, ensuring it works alongside the existing completion endpoint.

## Background
The client needs to support both endpoints simultaneously, allowing users to choose between the old prompt-based and new chat-based approaches.

## Tasks

### 1. Update Client Implementation
**File**: `client/src/client.rs`

**Add Chat Endpoint Method**:
```rust
impl StraicoClient {
    /// Creates a request builder for the new chat completions endpoint
    pub fn chat_completions<'a>(self) -> StraicoRequestBuilder<NoApiKey, ChatRequest> {
        self.0
            .post("https://api.straico.com/v0/chat/completions")
            .into()
    }
    
    // Keep existing completion() method unchanged
    pub fn completion<'a>(self) -> StraicoRequestBuilder<NoApiKey, CompletionRequest<'a>> {
        self.0
            .post("https://api.straico.com/v1/prompt/completion")
            .into()
    }
}
```

### 2. Update Request Builder for Chat
**File**: `client/src/client.rs`

**Add Chat-Specific Response Handling**:
```rust
impl StraicoRequestBuilder<ApiKeySet, PayloadSet> {
    /// Sends chat request and returns chat response
    pub async fn send_chat(self) -> Result<ChatApiResponseData, StraicoError> {
        let response = self.0.send().await?;
        let json = response.json().await?;
        Ok(json)
    }
    
    // Keep existing send() method for completion endpoint
}
```

### 3. Create Chat Response Wrapper
**File**: `client/src/endpoints/chat/chat_response.rs`

**Add API Response Wrapper**:
```rust
use crate::error::StraicoError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ChatApiResponseData {
    success: bool,
    #[serde(flatten)]
    response: ChatApiResponseVariant,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum ChatApiResponseVariant {
    Error { error: String },
    Data { data: ChatResponse },
}

impl ChatApiResponseData {
    pub fn get_chat_response(self) -> Result<ChatResponse, StraicoError> {
        match self.response {
            ChatApiResponseVariant::Data { data } => Ok(data),
            ChatApiResponseVariant::Error { error } => Err(StraicoError::Api(error)),
        }
    }
}
```

### 4. Update Error Handling
**File**: `client/src/error.rs`

**Ensure Chat Errors are Handled**:
```rust
// Verify existing StraicoError enum covers chat endpoint errors
// Add new variants if needed
```

### 5. Add Integration Tests
**File**: `client/src/endpoints/chat/mod.rs`

**Basic Integration Tests**:
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::client::StraicoClient;

    #[tokio::test]
    async fn test_chat_endpoint_structure() {
        let client = StraicoClient::new();
        let request_builder = client.chat_completions();
        
        // Test that the request builder is created correctly
        // Note: Don't make actual API calls in unit tests
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: vec![ContentObject {
                        content_type: "text".to_string(),
                        text: "Hello".to_string(),
                    }],
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(150),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-model"));
        assert!(json.contains("Hello"));
    }
}
```

### 6. Update Documentation
**File**: `client/README.md` (if exists) or create `client/CHAT_ENDPOINT.md`

**Usage Examples**:
```rust
// Basic chat completion
let client = StraicoClient::new();
let response = client
    .chat_completions()
    .bearer_auth("your-api-key")
    .json(ChatRequest {
        model: "meta-llama/llama-4-maverick".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: vec![ContentObject {
                    content_type: "text".to_string(),
                    text: "Hello, how are you?".to_string(),
                }],
            }
        ],
        temperature: Some(0.7),
        max_tokens: Some(150),
    })
    .send_chat()
    .await?
    .get_chat_response()?;
```

## Deliverables

1. **Updated Client**:
   - New `chat_completions()` method
   - Chat-specific response handling
   - Preserved existing completion endpoint

2. **Response Handling**:
   - Chat API response wrapper
   - Error handling integration
   - Response parsing utilities

3. **Tests**:
   - Integration test structure
   - Serialization tests
   - Basic functionality tests

4. **Documentation**:
   - Usage examples
   - API differences explanation
   - Migration guidance

## Success Criteria

- [ ] Client has new chat_completions() method
- [ ] Chat requests can be built and serialized
- [ ] Response handling works for chat format
- [ ] Existing completion endpoint unchanged
- [ ] Integration tests pass
- [ ] Documentation is clear and helpful
- [ ] Code compiles without errors

## Time Estimate
**Duration**: 2-3 hours

## Dependencies
- **P1-T2**: Create New Chat Request/Response Structures
- **P1-T3**: Implement Content Format Conversion

## Next Task
**P1-T5**: Update Proxy Server for New Endpoint

## Notes
- Keep both endpoints working simultaneously
- Use different method names to avoid confusion
- Ensure error handling is consistent between endpoints
- Consider adding feature flags for endpoint selection