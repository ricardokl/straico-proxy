# P2-T6: Testing and Validation (Phase 2)

## Objective
Thoroughly test the tool calling implementation to ensure XML embedding and parsing works correctly with the new chat endpoint.

## Background
Phase 2 adds tool calling functionality via XML embedding in user messages. This needs comprehensive testing to ensure compatibility with existing tool parsing and proper OpenAI API compliance.

## Tasks

### 1. Unit Tests for Tool Embedding
**File**: `proxy/src/tool_embedding.rs` (extend with tests)

**Test Cases**:
```rust
#[test]
fn test_embed_single_tool() {
    // Test embedding one tool in user message
}

#[test]
fn test_embed_multiple_tools() {
    // Test embedding multiple tools
}

#[test]
fn test_embed_with_string_content() {
    // Test tool embedding with string user content
}

#[test]
fn test_embed_with_array_content() {
    // Test tool embedding with array user content
}

#[test]
fn test_model_specific_formatting() {
    // Test different XML formats for different models
}

#[test]
fn test_no_tools_passthrough() {
    // Test that messages without tools pass through unchanged
}
```

### 2. Integration Tests for Tool Calls
**File**: `proxy/tests/tool_calling_tests.rs` (new file)

**Test Scenarios**:
- Complete tool call flow (request → embedding → response → parsing)
- Multiple tool calls in single response
- Tool calls with different models
- Error handling for malformed tool responses
- Tool response processing

### 3. OpenAI Tool API Compatibility Tests
**File**: `proxy/tests/openai_tool_compatibility_tests.rs` (new file)

**Compatibility Tests**:
- Tool definition format matches OpenAI spec
- Tool call response format matches OpenAI spec
- Function calling parameter handling
- Tool choice parameter support
- Error responses for tool-related errors

### 4. Manual Tool Testing Script
**File**: `scripts/test_tool_calling.sh` (new file)

**Test Script**:
```bash
#!/bin/bash
# Manual testing script for tool calling

echo "Testing basic tool call..."
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "test-model",
    "messages": [{"role": "user", "content": "What is the weather?"}],
    "tools": [{
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get current weather",
        "parameters": {
          "type": "object",
          "properties": {
            "location": {"type": "string"}
          }
        }
      }
    }]
  }'

echo "Testing tool response..."
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "test-model",
    "messages": [
      {"role": "user", "content": "What is the weather?"},
      {"role": "assistant", "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "get_weather", "arguments": "{\"location\": \"NYC\"}"}}]},
      {"role": "tool", "tool_call_id": "call_1", "content": "Sunny, 72°F"}
    ]
  }'
```

### 5. Tool Response Parsing Tests
**File**: `client/tests/tool_parsing_tests.rs` (new file)

**Parsing Tests**:
- XML tool call extraction
- JSON parsing from tool calls
- Model-specific format handling
- Error handling for malformed XML
- Multiple tool calls in single response

### 6. Performance Testing with Tools
**Test Scenarios**:
- Response time with tool embedding
- Memory usage with large tool definitions
- Concurrent requests with tools
- Large tool response processing

### 7. Error Handling Validation
**Error Scenarios**:
- Invalid tool definitions
- Malformed tool responses from Straico
- Missing tool call IDs
- Invalid JSON in tool arguments
- Tool call timeout scenarios

## Deliverables

1. **Comprehensive Test Suite**:
   - Unit tests for tool embedding logic
   - Integration tests for complete tool flow
   - OpenAI compatibility validation
   - Performance benchmarks with tools

2. **Testing Scripts**:
   - Manual tool testing script
   - Automated tool test runner
   - Tool performance measurement

3. **Test Documentation**:
   - Tool test coverage report
   - Known limitations with tools
   - Performance impact analysis

## Success Criteria

- [ ] All tool embedding unit tests pass
- [ ] Integration tests cover tool call scenarios
- [ ] OpenAI tool API compatibility verified
- [ ] Manual testing script works with real tools
- [ ] Tool response parsing works correctly
- [ ] Performance impact acceptable
- [ ] Error handling robust for tool scenarios
- [ ] Test coverage > 85% for tool-related code

## Time Estimate
**Duration**: 4-5 hours

## Dependencies
- **P2-T5**: Update OpenAI Compatibility Layer

## Next Task
**P3-T1**: Analyze Current Streaming Implementation

## Notes
- Focus on edge cases in tool XML parsing
- Test with various model-specific formats
- Ensure tool call IDs are handled correctly
- Document any tool-related limitations discovered
- Verify tool calls work with both content formats