# Tool Adaptation Strategy

## Overview

This document outlines the strategy for adapting the current tool implementation to work with the new chat endpoint's structured message format.

## Current Implementation Analysis

The current tool implementation embeds tool definitions in system messages using XML format:
- Tools are serialized as JSON and wrapped in `<tools></tools>` tags
- Additional instructions are provided for tool calling with model-specific XML tags
- The embedding happens in the `Chat.to_prompt()` method
- Tool calls in responses are parsed using regex pattern matching

## Adaptation Requirements

For the new chat endpoint:
- Tool definitions need to be embedded in user messages instead of system messages
- The embedding should work with both string and array content formats
- Existing XML generation and parsing logic should be preserved
- Model-specific formatting must be maintained

## Strategy Options

### 1. First User Message Embedding
**Approach**: Embed tools in the first user message
- Prepend tool XML to the user's actual content
- Maintain existing XML format for compatibility
- Simple implementation with clear logic

**Pros**:
- Minimal changes to existing logic
- Clear and predictable behavior
- Maintains compatibility with existing parsing

**Cons**:
- Only works with the first user message
- May affect user content presentation

### 2. Separate Tool Message
**Approach**: Add a dedicated tool definition message
- Insert a separate message containing only tool definitions
- Position before the first user message

**Pros**:
- Clean separation of concerns
- No content mixing

**Cons**:
- More complex implementation
- May not work with all models
- Requires changes to message ordering logic

### 3. Per-Message Tools
**Approach**: Embed tools with each relevant message
- Add tool definitions to every message that might need them

**Pros**:
- Maximum flexibility
- Works with complex conversation flows

**Cons**:
- Redundant tool definitions
- Increased token usage
- Complex implementation

## Recommended Approach

**First User Message Embedding** is the recommended approach because:

1. **Simplicity**: Minimal changes to existing implementation
2. **Compatibility**: Preserves existing XML format and parsing logic
3. **Efficiency**: No redundant tool definitions
4. **Predictability**: Clear and consistent behavior

## Implementation Plan

### 1. Tool Embedding Location
- Move from system message to first user message
- Preserve all existing XML generation logic
- Maintain model-specific formatting

### 2. Content Merging Logic
- Handle both string and array content formats
- Prepend tool XML to user content
- Preserve original user message structure

### 3. Message Processing Flow
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

### 4. Backward Compatibility
- Existing tool parsing logic remains unchanged
- Same XML format as current implementation
- Same model-specific formatting
- Same error handling patterns

## Risk Assessment

### Low Risk Factors
- Existing well-tested XML generation logic
- Proven parsing and error handling
- Minimal architectural changes

### Mitigation Strategies
- Comprehensive unit testing
- Model-specific validation
- Error handling for edge cases

## Summary

The First User Message Embedding approach provides the best balance of simplicity, compatibility, and maintainability. It leverages the existing robust tool implementation while adapting it to the new chat endpoint requirements with minimal changes.