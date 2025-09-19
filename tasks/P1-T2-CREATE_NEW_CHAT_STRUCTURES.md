# P1-T2: Create New Chat Request/Response Structures

## Objective
Create new data structures to handle the new Straico chat endpoint format while maintaining compatibility with existing code patterns.

## Background
The new chat endpoint uses a different request/response format:
- Single model instead of multiple models
- Messages with content arrays instead of formatted prompts
- Different response structure (to be determined)

## Tasks

### 1. Create New Chat Request Structure
**File**: `client/src/endpoints/chat/mod.rs`

**New Module Structure**:
```
client/src/endpoints/
├── completion/          # Existing prompt endpoint
│   ├── completion_request.rs
│   ├── completion_response.rs
│   └── mod.rs
└── chat/               # New chat endpoint
    ├── chat_request.rs
    ├── chat_response.rs
    └── mod.rs
```

**Chat Request Structure**:
```rust
#[derive(Serialize, Debug)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Serialize, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<ContentObject>,
}

#[derive(Serialize, Debug)]
pub struct ContentObject {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}
```

### 2. Create New Chat Response Structure
**File**: `client/src/endpoints/chat/chat_response.rs`

**Initial Response Structure** (to be refined based on actual API response):
```rust
#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    pub model: String,
    pub usage: Option<ChatUsage>,
}

#[derive(Deserialize, Debug)]
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: String,
}

#[derive(Deserialize, Debug)]
pub struct ChatResponseMessage {
    pub role: String,
    pub content: String,
}
```

### 3. Add Chat Endpoint to Client
**File**: `client/src/client.rs`

**New Method**:
```rust
impl StraicoClient {
    /// Creates a request builder for the new chat endpoint
    pub fn chat<'a>(self) -> StraicoRequestBuilder<NoApiKey, ChatRequest> {
        self.0
            .post("https://api.straico.com/v0/chat/completions")
            .into()
    }
}
```

### 4. Update Module Exports
**File**: `client/src/endpoints/mod.rs`

**Add**:
```rust
pub mod chat;
pub mod completion; // existing
```

**File**: `client/src/lib.rs`

**Update exports to include new chat structures**

### 5. Create Conversion Utilities
**File**: `client/src/endpoints/chat/conversions.rs`

**Content Format Conversion**:
```rust
impl From<String> for Vec<ContentObject> {
    fn from(text: String) -> Self {
        vec![ContentObject {
            content_type: "text".to_string(),
            text,
        }]
    }
}

impl From<Vec<ContentObject>> for Vec<ContentObject> {
    fn from(content: Vec<ContentObject>) -> Self {
        content
    }
}
```

**OpenAI to Chat Conversion**:
```rust
impl From<OpenAiChatMessage> for ChatMessage {
    fn from(msg: OpenAiChatMessage) -> Self {
        // Handle both string and array content formats
    }
}
```

## Deliverables

1. **New Module Structure**:
   - `client/src/endpoints/chat/mod.rs`
   - `client/src/endpoints/chat/chat_request.rs`
   - `client/src/endpoints/chat/chat_response.rs`
   - `client/src/endpoints/chat/conversions.rs`

2. **Updated Client**:
   - New `chat()` method in `StraicoClient`
   - Updated module exports

3. **Conversion Utilities**:
   - Content format conversion functions
   - OpenAI compatibility helpers

## Success Criteria

- [ ] New chat module structure created
- [ ] ChatRequest structure handles new format
- [ ] ChatResponse structure ready for API responses
- [ ] Client has new chat() method
- [ ] Content format conversion utilities work
- [ ] Code compiles without errors
- [ ] Basic unit tests pass

## Time Estimate
**Duration**: 3-4 hours

## Dependencies
- **P1-T1**: Preserve Current Implementation (for reference patterns)

## Next Task
**P1-T3**: Implement Content Format Conversion

## Notes
- Start with minimal structures and expand as needed
- Keep similar patterns to existing completion structures
- Response structure may need adjustment after testing with real API
- Focus on compilation and basic functionality first