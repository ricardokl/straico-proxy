# P2-T2: Design Tool Embedding Strategy

## Objective
Design the strategy for embedding tool definitions in user messages for the new chat endpoint, ensuring compatibility with existing tool parsing logic.

## Background
Based on the analysis from P2-T1, design how to adapt the current tool embedding approach to work with the new chat endpoint's structured message format.

## Tasks

### 1. Define Tool Embedding Approach
**File**: `TOOL_EMBEDDING_DESIGN.md`

**Design Decision**: First User Message Embedding
- Embed tool definitions in the first user message
- Prepend tool XML to user's actual content
- Maintain existing XML format for compatibility

**Message Structure**:
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

### 2. Design Tool Definition Injection
**File**: `proxy/src/tool_embedding.rs` (new file design)

**Core Functions**:
```rust
pub fn embed_tools_in_first_message(
    messages: &mut Vec<ChatMessage>,
    tools: Option<Vec<Tool>>
) -> Result<(), ToolEmbeddingError> {
    // Find first user message
    // Generate tool XML
    // Prepend to message content
}

pub fn generate_tool_xml(tools: &[Tool], model: &str) -> String {
    // Reuse existing logic from chat.rs
    // Generate model-specific XML format
}

pub fn extract_user_content_from_embedded(
    content: &str
) -> (Option<String>, String) {
    // Separate tool XML from user content
    // Return (tool_xml, user_content)
}
```

### 3. Design Content Merging Logic
**Strategy for Content Combination**:
- Tool XML + user content in single text object
- Preserve original user message structure
- Handle both string and array content inputs

**Content Processing Flow**:
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

### 4. Design Backward Compatibility
**Compatibility Requirements**:
- Existing tool parsing logic unchanged
- Same XML format as current implementation
- Same model-specific formatting
- Same error handling patterns

**Adaptation Points**:
- Tool embedding location (system → user message)
- Content structure (prompt → message array)
- Request building process

### 5. Design Error Handling
**Error Scenarios**:
- No user messages in request
- Empty tool definitions
- Invalid tool schemas
- Content merging failures

**Error Handling Strategy**:
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

### 6. Design Testing Strategy
**Test Cases**:
- Tool embedding with string content
- Tool embedding with array content
- Multiple tools embedding
- No tools (passthrough)
- Error conditions

**Test Structure**:
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

## Deliverables

1. **Design Document**:
   - `TOOL_EMBEDDING_DESIGN.md` - Complete design specification

2. **Module Design**:
   - `proxy/src/tool_embedding.rs` - Function signatures and logic design
   - Error types and handling strategy
   - Test case specifications

3. **Integration Plan**:
   - How tool embedding integrates with content conversion
   - How it fits into the request processing pipeline
   - Compatibility with existing parsing logic

## Success Criteria

- [ ] Tool embedding strategy clearly defined
- [ ] Content merging approach designed
- [ ] Error handling strategy complete
- [ ] Backward compatibility ensured
- [ ] Testing strategy defined
- [ ] Integration points identified
- [ ] Implementation plan ready

## Time Estimate
**Duration**: 2-3 hours

## Dependencies
- **P2-T1**: Analyze Current Tool Implementation

## Next Task
**P2-T3**: Implement Tool Definition Injection

## Notes
- Prioritize compatibility with existing tool parsing
- Keep the design simple and testable
- Consider future extensibility for other embedding strategies
- Ensure the approach works with all supported models