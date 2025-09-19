# P2-T4: Adapt Tool Response Parsing

## Objective
Adapt the existing tool response parsing logic to work with responses from the new chat endpoint, ensuring tool calls are correctly extracted and converted to OpenAI format.

## Background
The current tool parsing logic in `completion_response.rs` needs to be adapted to work with the new chat endpoint response format while maintaining the same XML parsing capabilities.

## Tasks

### 1. Analyze New Response Format
**Investigation**: Understand new chat endpoint response structure
- Test actual API responses from new endpoint
- Document response format differences
- Identify where tool calls appear in responses

**Expected Response Structure** (to be confirmed):
```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "Response with <tool_call>...</tool_call> XML"
    },
    "finish_reason": "stop"
  }],
  "model": "...",
  "usage": {...}
}
```

### 2. Create Chat Response Structures
**File**: `client/src/endpoints/chat/chat_response.rs` (update existing)

**Enhanced Response Structures**:
```rust
use straico_client::endpoints::completion::completion_response::ToolCall;

#[derive(Deserialize, Debug, Clone)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    pub model: String,
    pub usage: Option<ChatUsage>,
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: String,
    pub index: Option<u8>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatResponseMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}
```

### 3. Implement Tool Parsing for Chat Responses
**File**: `client/src/endpoints/chat/tool_parsing.rs` (new file)

**Core Parsing Logic**:
```rust
use crate::endpoints::completion::completion_response::ToolCall;
use crate::error::StraicoError;
use super::chat_response::{ChatResponse, ChatResponseMessage};

impl ChatResponseMessage {
    /// Parse tool calls from message content using existing XML parsing logic
    pub fn parse_tool_calls(&mut self, model: &str) -> Result<(), StraicoError> {
        // Reuse existing tool_calls_response logic from completion_response.rs
        if let Some(content) = &self.content {
            let tool_calls = extract_tool_calls_from_content(content, model)?;
            if !tool_calls.is_empty() {
                self.tool_calls = Some(tool_calls);
                self.content = None; // Remove content when tool calls found
            }
        }
        Ok(())
    }
}

impl ChatResponse {
    /// Parse tool calls from all choices in the response
    pub fn parse_tool_calls(&mut self, model: &str) -> Result<(), StraicoError> {
        for choice in &mut self.choices {
            choice.message.parse_tool_calls(model)?;
            
            // Update finish_reason if tool calls found
            if choice.message.tool_calls.is_some() {
                choice.finish_reason = "tool_calls".to_string();
            } else if choice.finish_reason == "end_turn" {
                choice.finish_reason = "stop".to_string();
            }
        }
        Ok(())
    }
}

fn extract_tool_calls_from_content(
    content: &str,
    model: &str,
) -> Result<Vec<ToolCall>, StraicoError> {
    // Reuse existing XML parsing logic from completion_response.rs
    // This is the same logic as tool_calls_response() but extracted
    
    let format = get_prompt_format_for_model(model);
    
    if content.find(&format.tool_calls.tool_call_begin).is_some()
        || content.find(&format.tool_calls.tool_call_end).is_some()
    {
        let pattern = format!(
            r"{}(.*?){}",
            regex::escape(&format.tool_calls.tool_call_begin),
            regex::escape(&format.tool_calls.tool_call_end)
        );
        
        let re = regex::Regex::new(&pattern)?;
        let tool_calls = re
            .find_iter(&content.replace("\n", ""))
            .map(|c| {
                c.as_str()
                    .trim_start_matches(&format.tool_calls.tool_call_begin)
                    .trim_end_matches(&format.tool_calls.tool_call_end)
            })
            .map(|s| {
                serde_json::from_str::<FunctionData>(s).map(|function_data| {
                    ToolCall::Function {
                        id: String::from("func"),
                        function: function_data,
                    }
                })
            })
            .collect::<Result<Vec<ToolCall>, _>>()?;
            
        Ok(tool_calls)
    } else {
        Ok(vec![])
    }
}
```

### 4. Create Response Conversion Utilities
**File**: `proxy/src/chat_response_conversion.rs` (new file)

**Convert Chat Response to OpenAI Format**:
```rust
use straico_client::endpoints::chat::ChatResponse;
use serde_json::Value;

pub fn convert_chat_response_to_openai(
    chat_response: ChatResponse,
    request_id: &str,
) -> Value {
    serde_json::json!({
        "id": chat_response.id.unwrap_or_else(|| format!("chatcmpl-{}", request_id)),
        "object": "chat.completion",
        "created": chat_response.created.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        }),
        "model": chat_response.model,
        "choices": chat_response.choices.into_iter().map(|choice| {
            serde_json::json!({
                "index": choice.index.unwrap_or(0),
                "message": {
                    "role": choice.message.role,
                    "content": choice.message.content,
                    "tool_calls": choice.message.tool_calls
                },
                "finish_reason": choice.finish_reason
            })
        }).collect::<Vec<_>>(),
        "usage": chat_response.usage.map(|usage| {
            serde_json::json!({
                "prompt_tokens": usage.prompt_tokens.unwrap_or(0),
                "completion_tokens": usage.completion_tokens.unwrap_or(0),
                "total_tokens": usage.total_tokens.unwrap_or(0)
            })
        })
    })
}
```

### 5. Update Client for Chat Response Handling
**File**: `client/src/client.rs`

**Add Chat Response Handling**:
```rust
impl StraicoRequestBuilder<ApiKeySet, PayloadSet> {
    /// Send chat request and parse tool calls from response
    pub async fn send_chat(self) -> Result<ChatResponse, StraicoError> {
        let response = self.0.send().await?;
        let mut chat_response: ChatResponse = response.json().await?;
        
        // Parse tool calls from response content
        // Note: We need the model name for parsing, might need to pass it separately
        // chat_response.parse_tool_calls(model_name)?;
        
        Ok(chat_response)
    }
}
```

### 6. Add Unit Tests
**File**: `client/src/endpoints/chat/tool_parsing.rs`

**Test Cases**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls_from_content() {
        let content = r#"Here's the result: <tool_call>{"name": "test_func", "arguments": {"param": "value"}}</tool_call>"#;
        let tool_calls = extract_tool_calls_from_content(content, "test-model").unwrap();
        
        assert_eq!(tool_calls.len(), 1);
        match &tool_calls[0] {
            ToolCall::Function { function, .. } => {
                assert_eq!(function.name, "test_func");
            }
        }
    }

    #[test]
    fn test_no_tool_calls_in_content() {
        let content = "Just a regular response";
        let tool_calls = extract_tool_calls_from_content(content, "test-model").unwrap();
        assert!(tool_calls.is_empty());
    }

    #[test]
    fn test_chat_response_tool_parsing() {
        let mut response = ChatResponse {
            choices: vec![ChatChoice {
                message: ChatResponseMessage {
                    role: "assistant".to_string(),
                    content: Some("<tool_call>{\"name\": \"test\"}</tool_call>".to_string()),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
                index: Some(0),
            }],
            model: "test-model".to_string(),
            usage: None,
            id: None,
            object: None,
            created: None,
        };

        response.parse_tool_calls("test-model").unwrap();
        
        assert!(response.choices[0].message.tool_calls.is_some());
        assert!(response.choices[0].message.content.is_none());
        assert_eq!(response.choices[0].finish_reason, "tool_calls");
    }
}
```

## Deliverables

1. **Enhanced Response Structures**:
   - Updated `ChatResponse` with tool call support
   - Tool parsing methods for chat responses

2. **Tool Parsing Logic**:
   - Extracted and adapted XML parsing logic
   - Model-specific format handling
   - Error handling for malformed responses

3. **Conversion Utilities**:
   - Chat response to OpenAI format conversion
   - Proper field mapping and defaults

4. **Tests**:
   - Unit tests for tool parsing
   - Response conversion tests
   - Edge case handling

## Success Criteria

- [ ] Chat response structures support tool calls
- [ ] Tool parsing works with new response format
- [ ] XML parsing logic correctly adapted
- [ ] Response conversion to OpenAI format works
- [ ] Finish reason updates correctly for tool calls
- [ ] All unit tests pass
- [ ] Error handling works for malformed responses

## Time Estimate
**Duration**: 3-4 hours

## Dependencies
- **P2-T3**: Implement Tool Definition Injection

## Next Task
**P2-T5**: Update OpenAI Compatibility Layer

## Notes
- Reuse existing XML parsing logic as much as possible
- Ensure model-specific formatting is preserved
- Test with actual API responses when available
- Focus on maintaining compatibility with existing tool call format