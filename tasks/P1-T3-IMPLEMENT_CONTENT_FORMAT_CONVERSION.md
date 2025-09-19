# P1-T3: Implement Content Format Conversion

## Objective
Implement robust content format conversion to handle both string and array content formats from OpenAI-style requests, converting them to the new Straico chat format.

## Background
The proxy needs to handle two content formats from OpenAI requests:
1. **String format**: `"content": "Hello world"`
2. **Array format**: `"content": [{"type": "text", "text": "Hello world"}]`

Both need to be converted to the new Straico format: `[{"type": "text", "text": "..."}]`

## Tasks

### 1. Define OpenAI Content Types
**File**: `proxy/src/openai_types.rs` (new file)

**Content Structures**:
```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum OpenAiContent {
    String(String),
    Array(Vec<OpenAiContentObject>),
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAiContentObject {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAiChatMessage {
    pub role: String,
    pub content: OpenAiContent,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::Tool>>, // Reference existing Tool type
}
```

### 2. Implement Content Conversion Logic
**File**: `proxy/src/content_conversion.rs` (new file)

**Conversion Functions**:
```rust
use straico_client::endpoints::chat::{ChatMessage, ContentObject};
use crate::openai_types::{OpenAiContent, OpenAiChatMessage};

impl From<OpenAiContent> for Vec<ContentObject> {
    fn from(content: OpenAiContent) -> Self {
        match content {
            OpenAiContent::String(text) => {
                vec![ContentObject {
                    content_type: "text".to_string(),
                    text,
                }]
            }
            OpenAiContent::Array(objects) => {
                objects.into_iter().map(|obj| ContentObject {
                    content_type: obj.content_type,
                    text: obj.text,
                }).collect()
            }
        }
    }
}

impl From<OpenAiChatMessage> for ChatMessage {
    fn from(msg: OpenAiChatMessage) -> Self {
        ChatMessage {
            role: msg.role,
            content: msg.content.into(),
        }
    }
}

pub fn convert_openai_to_chat_request(
    openai_req: OpenAiChatRequest
) -> straico_client::endpoints::chat::ChatRequest {
    straico_client::endpoints::chat::ChatRequest {
        model: openai_req.model,
        messages: openai_req.messages.into_iter().map(Into::into).collect(),
        temperature: openai_req.temperature,
        max_tokens: openai_req.max_tokens,
    }
}
```

### 3. Add Content Validation
**File**: `proxy/src/content_conversion.rs`

**Validation Functions**:
```rust
pub fn validate_content_objects(content: &[ContentObject]) -> Result<(), String> {
    for obj in content {
        if obj.content_type != "text" {
            return Err(format!("Unsupported content type: {}", obj.content_type));
        }
        if obj.text.is_empty() {
            return Err("Empty text content not allowed".to_string());
        }
    }
    Ok(())
}

pub fn validate_openai_content(content: &OpenAiContent) -> Result<(), String> {
    match content {
        OpenAiContent::String(text) => {
            if text.is_empty() {
                return Err("Empty string content not allowed".to_string());
            }
        }
        OpenAiContent::Array(objects) => {
            if objects.is_empty() {
                return Err("Empty content array not allowed".to_string());
            }
            for obj in objects {
                if obj.content_type != "text" {
                    return Err(format!("Unsupported content type: {}", obj.content_type));
                }
                if obj.text.is_empty() {
                    return Err("Empty text in content object".to_string());
                }
            }
        }
    }
    Ok(())
}
```

### 4. Create Unit Tests
**File**: `proxy/src/content_conversion.rs`

**Test Cases**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_content_conversion() {
        let content = OpenAiContent::String("Hello world".to_string());
        let converted: Vec<ContentObject> = content.into();
        
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content_type, "text");
        assert_eq!(converted[0].text, "Hello world");
    }

    #[test]
    fn test_array_content_conversion() {
        let content = OpenAiContent::Array(vec![
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "Hello".to_string(),
            },
            OpenAiContentObject {
                content_type: "text".to_string(),
                text: "World".to_string(),
            },
        ]);
        let converted: Vec<ContentObject> = content.into();
        
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].text, "Hello");
        assert_eq!(converted[1].text, "World");
    }

    #[test]
    fn test_message_conversion() {
        let msg = OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("Test message".to_string()),
        };
        let converted: ChatMessage = msg.into();
        
        assert_eq!(converted.role, "user");
        assert_eq!(converted.content.len(), 1);
        assert_eq!(converted.content[0].text, "Test message");
    }

    #[test]
    fn test_validation_success() {
        let content = OpenAiContent::String("Valid content".to_string());
        assert!(validate_openai_content(&content).is_ok());
    }

    #[test]
    fn test_validation_empty_string() {
        let content = OpenAiContent::String("".to_string());
        assert!(validate_openai_content(&content).is_err());
    }
}
```

### 5. Update Module Structure
**File**: `proxy/src/main.rs`

**Add modules**:
```rust
mod openai_types;
mod content_conversion;
```

**File**: `proxy/src/lib.rs` (if it exists)

**Export new modules**

## Deliverables

1. **New Files**:
   - `proxy/src/openai_types.rs` - OpenAI request/response types
   - `proxy/src/content_conversion.rs` - Conversion logic and tests

2. **Conversion Functions**:
   - String to ContentObject array conversion
   - Array to ContentObject array conversion
   - OpenAI message to Chat message conversion
   - Full request conversion function

3. **Validation**:
   - Content format validation
   - Error handling for invalid content

4. **Tests**:
   - Unit tests for all conversion functions
   - Validation test cases
   - Edge case handling

## Success Criteria

- [ ] OpenAI content types defined and working
- [ ] String content converts to array format correctly
- [ ] Array content passes through correctly
- [ ] Message conversion handles all fields
- [ ] Request conversion works end-to-end
- [ ] Validation catches invalid content
- [ ] All unit tests pass
- [ ] Code compiles without warnings

## Time Estimate
**Duration**: 2-3 hours

## Dependencies
- **P1-T2**: Create New Chat Request/Response Structures

## Next Task
**P1-T4**: Add New Endpoint to Client

## Notes
- Focus on robust error handling for malformed content
- Ensure validation is comprehensive but not overly restrictive
- Keep conversion logic simple and testable
- Consider future extensibility for other content types