# Tool Embedding Design

## Overview

This document specifies the design for embedding tool definitions in user messages for the new chat endpoint, ensuring compatibility with existing tool parsing logic.

## Tool Embedding Approach

### Design Decision: First User Message Embedding
- Embed tool definitions in the first user message
- Prepend tool XML to user's actual content
- Maintain existing XML format for compatibility

### Message Structure
```json
{
  "role": "user",
  "content": [
    {
      "type": "text", 
      "text": "<tools>...</tools>\n\nUser's actual message"
    }
  ]
}
```

## Tool Embedding Module Design

### File: `proxy/src/tool_embedding.rs`

### Core Functions

```rust
use straico_client::chat::Tool;
use crate::openai_types::{OpenAiChatMessage, OpenAiContent};
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
    // Implementation will find first user message and embed tools
}

pub fn generate_tool_xml(tools: &[Tool], model: &str) -> String {
    // Reuse existing logic from client/src/chat.rs
    // Generate model-specific XML format
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

## Content Merging Logic

### Strategy for Content Combination
- Tool XML + user content in single text object
- Preserve original user message structure
- Handle both string and array content inputs

### Content Processing Flow
```
OpenAI Request with tools + messages
    ↓
Extract first user message
    ↓
Generate tool XML for model
    ↓
Merge: tool_xml + "\n\n" + user_content
    ↓
Create new ChatMessage with merged content
    ↓
Send to Straico API
```

## Backward Compatibility

### Compatibility Requirements
- Existing tool parsing logic unchanged
- Same XML format as current implementation
- Same model-specific formatting
- Same error handling patterns

### Adaptation Points
- Tool embedding location (system → user message)
- Content structure (prompt → message array)
- Request building process

## Error Handling

### Error Scenarios
- No user messages in request
- Empty tool definitions
- Invalid tool schemas
- Content merging failures

### Error Handling Strategy
```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolEmbeddingError {
    #[error("No user messages found for tool embedding")]
    NoUserMessages,
    #[error("Invalid tool definition: {0}")]
    InvalidTool(String),
    #[error("Content merging failed: {0}")]
    ContentMerging(String),
}
```

## Testing Strategy

### Test Cases
- Tool embedding with string content
- Tool embedding with array content
- Multiple tools embedding
- No tools (passthrough)
- Error conditions

### Test Structure
```rust
#[test]
fn test_embed_tools_string_content() {
    // Test tool embedding with string user content
}

#[test]
fn test_embed_tools_array_content() {
    // Test tool embedding with array user content
}

#[test]
fn test_no_user_messages_error() {
    // Test error when no user messages present
}
```

## Integration Plan

### Integration Points
1. Content conversion module - where tool embedding will be called
2. OpenAI request processing - before converting to Straico format
3. Error handling - consistent with existing patterns

### Implementation Sequence
1. Create `tool_embedding.rs` module
2. Implement core functions
3. Add unit tests
4. Integrate with content conversion pipeline
5. Validate with existing test suite