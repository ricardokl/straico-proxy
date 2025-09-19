# P2-T3: Implement Tool Definition Injection

## Objective
Implement the tool definition injection logic that embeds tool XML into the first user message for the new chat endpoint.

## Background
Based on the design from P2-T2, implement the actual code that takes OpenAI-style tool definitions and embeds them as XML in the first user message content.

## Tasks

### 1. Create Tool Embedding Module
**File**: `proxy/src/tool_embedding.rs` (new file)

**Core Implementation**:
```rust
use straico_client::chat::Tool;
use crate::openai_types::{OpenAiChatMessage, OpenAiContent};
use crate::content_conversion::ContentObject;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolEmbeddingError {
    #[error("No user messages found for tool embedding")]
    NoUserMessages,
    #[error("Invalid tool definition: {0}")]
    InvalidTool(String),
    #[error("Content merging failed: {0}")]
    ContentMerging(String),
}

pub fn embed_tools_in_messages(
    messages: &mut Vec<OpenAiChatMessage>,
    tools: Option<Vec<Tool>>,
    model: &str,
) -> Result<(), ToolEmbeddingError> {
    // Implementation here
}

pub fn generate_tool_xml(tools: &[Tool], model: &str) -> String {
    // Reuse logic from client/src/chat.rs
}

fn find_first_user_message_mut(
    messages: &mut [OpenAiChatMessage]
) -> Option<&mut OpenAiChatMessage> {
    // Find first user message
}

fn merge_tool_xml_with_content(
    tool_xml: &str,
    original_content: &OpenAiContent,
) -> Result<OpenAiContent, ToolEmbeddingError> {
    // Merge tool XML with user content
}
```

### 2. Implement Tool XML Generation
**Reuse Existing Logic**:
- Extract tool XML generation from `client/src/chat.rs`
- Adapt for standalone use
- Maintain model-specific formatting

**Implementation**:
```rust
pub fn generate_tool_xml(tools: &[Tool], model: &str) -> String {
    // Determine format based on model
    let format = get_prompt_format_for_model(model);
    
    let pre_tools = r###"
# Tools

You may call one or more functions to assist with the user query

You are provided with available function signatures within <tools></tools> XML tags:
<tools>
"###;

    let post_tools = format!(
        "\n</tools>\n# Tool Calls\n\nStart with the opening tag {}. For each tool call, return a json object with function name and arguments within {}{} tags:\n{}{{\"name\": <function-name>{} \"arguments\": <args-json-object>}}{}. close the tool calls section with {}\n",
        format.tool_calls.tool_calls_begin,
        format.tool_calls.tool_call_begin,
        format.tool_calls.tool_call_end,
        format.tool_calls.tool_call_begin,
        format.tool_calls.tool_sep,
        format.tool_calls.tool_call_end,
        format.tool_calls.tool_calls_end
    );

    let mut tools_message = String::new();
    tools_message.push_str(pre_tools);
    for tool in tools {
        tools_message.push_str(&serde_json::to_string_pretty(tool).unwrap());
    }
    tools_message.push_str(&post_tools);
    
    tools_message
}
```

### 3. Implement Content Merging
**Content Merging Logic**:
```rust
fn merge_tool_xml_with_content(
    tool_xml: &str,
    original_content: &OpenAiContent,
) -> Result<OpenAiContent, ToolEmbeddingError> {
    match original_content {
        OpenAiContent::String(text) => {
            let merged = format!("{}\n\n{}", tool_xml, text);
            Ok(OpenAiContent::String(merged))
        }
        OpenAiContent::Array(objects) => {
            let user_text = objects.iter()
                .filter(|obj| obj.content_type == "text")
                .map(|obj| obj.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            
            let merged = format!("{}\n\n{}", tool_xml, user_text);
            Ok(OpenAiContent::String(merged))
        }
    }
}
```

### 4. Implement Message Processing
**Main Function Implementation**:
```rust
pub fn embed_tools_in_messages(
    messages: &mut Vec<OpenAiChatMessage>,
    tools: Option<Vec<Tool>>,
    model: &str,
) -> Result<(), ToolEmbeddingError> {
    let tools = match tools {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(()), // No tools to embed
    };

    let first_user_msg = find_first_user_message_mut(messages)
        .ok_or(ToolEmbeddingError::NoUserMessages)?;

    let tool_xml = generate_tool_xml(&tools, model);
    let merged_content = merge_tool_xml_with_content(&tool_xml, &first_user_msg.content)?;
    
    first_user_msg.content = merged_content;
    Ok(())
}
```

### 5. Add Helper Functions
**Utility Functions**:
```rust
fn get_prompt_format_for_model(model: &str) -> PromptFormat<'static> {
    // Reuse model detection logic from chat.rs
    if model.to_lowercase().contains("anthropic") {
        ANTHROPIC_PROMPT_FORMAT
    } else if model.to_lowercase().contains("mistral") {
        MISTRAL_PROMPT_FORMAT
    } else if model.to_lowercase().contains("llama3") {
        LLAMA3_PROMPT_FORMAT
    } else if model.to_lowercase().contains("command") {
        COMMAND_R_PROMPT_FORMAT
    } else if model.to_lowercase().contains("qwen") {
        QWEN_PROMPT_FORMAT
    } else if model.to_lowercase().contains("deepseek") {
        DEEPSEEK_PROMPT_FORMAT
    } else {
        PromptFormat::default()
    }
}

fn find_first_user_message_mut(
    messages: &mut [OpenAiChatMessage]
) -> Option<&mut OpenAiChatMessage> {
    messages.iter_mut().find(|msg| msg.role == "user")
}
```

### 6. Add Unit Tests
**File**: `proxy/src/tool_embedding.rs`

**Test Cases**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_tools_string_content() {
        let mut messages = vec![OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("Hello".to_string()),
        }];
        
        let tools = vec![Tool::Function {
            name: "test_function".to_string(),
            description: Some("Test function".to_string()),
            parameters: None,
        }];

        embed_tools_in_messages(&mut messages, Some(tools), "test-model").unwrap();
        
        match &messages[0].content {
            OpenAiContent::String(content) => {
                assert!(content.contains("<tools>"));
                assert!(content.contains("test_function"));
                assert!(content.contains("Hello"));
            }
            _ => panic!("Expected string content"),
        }
    }

    #[test]
    fn test_no_tools_passthrough() {
        let mut messages = vec![OpenAiChatMessage {
            role: "user".to_string(),
            content: OpenAiContent::String("Hello".to_string()),
        }];
        let original = messages.clone();

        embed_tools_in_messages(&mut messages, None, "test-model").unwrap();
        
        assert_eq!(messages, original);
    }

    #[test]
    fn test_no_user_messages_error() {
        let mut messages = vec![OpenAiChatMessage {
            role: "system".to_string(),
            content: OpenAiContent::String("System message".to_string()),
        }];

        let tools = vec![Tool::Function {
            name: "test".to_string(),
            description: None,
            parameters: None,
        }];

        let result = embed_tools_in_messages(&mut messages, Some(tools), "test-model");
        assert!(matches!(result, Err(ToolEmbeddingError::NoUserMessages)));
    }
}
```

## Deliverables

1. **New Module**:
   - `proxy/src/tool_embedding.rs` - Complete implementation

2. **Core Functions**:
   - Tool XML generation
   - Content merging logic
   - Message processing pipeline
   - Error handling

3. **Tests**:
   - Unit tests for all functions
   - Edge case handling
   - Error condition testing

## Success Criteria

- [ ] Tool embedding module implemented
- [ ] Tool XML generation works correctly
- [ ] Content merging handles both string and array formats
- [ ] First user message correctly identified and modified
- [ ] Error handling works for edge cases
- [ ] All unit tests pass
- [ ] Code compiles without warnings

## Time Estimate
**Duration**: 3-4 hours

## Dependencies
- **P2-T2**: Design Tool Embedding Strategy

## Next Task
**P2-T4**: Adapt Tool Response Parsing

## Notes
- Reuse as much existing logic as possible from chat.rs
- Ensure model-specific formatting is preserved
- Focus on robust error handling
- Keep the implementation testable and modular