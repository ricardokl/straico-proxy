# Current Tool Flow Analysis

## Overview

This document analyzes the current tool implementation in the Straico proxy to understand how to adapt it for the new chat endpoint with XML embedding in user messages.

## Current Tool Flow

### 1. Tool Definition in OpenAI Requests
- Tools are defined in OpenAI requests as an array of function definitions with name, description, and parameters
- Tools are passed to the proxy server in the `OpenAiRequest` struct
- The tools field is optional and can be `None` or contain a vector of `Tool` structs

### 2. Tool Embedding in Straico Prompts
- In the current implementation, tools are embedded in system messages using XML format
- The embedding happens in the `Chat.to_prompt()` method in `client/src/chat.rs`
- Tool definitions are serialized as JSON and wrapped in `<tools></tools>` XML tags
- Model-specific formatting is applied to ensure compatibility with different LLMs
- The tool embedding includes instructions on how to call tools with proper XML tags

### 3. Tool Call Parsing from Responses
- Tool calls are parsed from responses in the `Message.tool_calls_response()` method in `client/src/endpoints/completion/completion_response.rs`
- The parsing looks for model-specific XML tags that wrap tool call JSON
- Regex patterns are used to extract tool call content from the assistant response
- Extracted JSON is parsed into `FunctionData` structs and converted to `ToolCall` instances
- The original content containing the tool calls is removed, and structured tool calls are stored

### 4. Tool Response Handling
- Tool responses are handled by creating Tool messages with the tool output content
- These are included in the message sequence for the next request
- The formatting ensures proper context for the model to continue the conversation

## Key Code Paths

```
OpenAI Request with tools
    ↓
proxy/src/server.rs (OpenAiRequest.tools)
    ↓
client/src/chat.rs (Chat.to_prompt with tools)
    ↓
Straico API call
    ↓
Response parsing (tool_calls_response)
    ↓
OpenAI-compatible response
```

## Analysis of Tool Embedding Logic (client/src/chat.rs, lines 304-338)

### Current Implementation
- Tool definition serialization using `serde_json::to_string_pretty(tool)`
- XML structure generation with pre-defined strings and model-specific formatting
- Model-specific formatting using constants like `ANTHROPIC_PROMPT_FORMAT`, `MISTRAL_PROMPT_FORMAT`, etc.
- System message injection by embedding tools in the first system message or creating one if none exists

### Key Questions for Adaptation
- The logic can work with user messages instead of system messages with minor modifications
- Multiple user messages with tools would require embedding in the first user message
- Changes needed: location of embedding (system → user message) and content merging logic

## Analysis of Tool Response Parsing

### Current Parsing Logic
- XML pattern matching using regex to find tool call sections
- JSON extraction from tool calls with model-specific format handling
- Error handling for malformed responses
- Conversion to structured `ToolCall` objects

### Adaptation Requirements
- The parsing logic should work with the new response format unchanged
- No changes needed for the parsing logic itself
- Error handling patterns can be preserved

## Reusable Components

### Components to Preserve
- Tool definition structures (`Tool` enum in `client/src/chat.rs`)
- XML formatting utilities and string constants
- Parsing regex patterns and logic
- Error handling patterns
- Model-specific format constants

### Components to Adapt
- Embedding location (system → user message)
- Message structure handling for the new chat format
- Content merging logic for array format content

## Summary

The current tool implementation is well-structured and modular, with clear separation between:
1. Tool definition and serialization
2. XML embedding and formatting
3. Response parsing and extraction
4. Model-specific adaptations

Most components are reusable, with the main adaptation being the location of tool embedding (from system message to first user message) and content merging logic for the new message format.