# Reusable Tool Components

## Overview

This document identifies the reusable components from the current tool implementation that can be leveraged for the new chat endpoint.

## Tool Definition Structures

### Tool Enum
- Located in `client/src/chat.rs`
- Represents function definitions with name, description, and parameters
- Supports JSON serialization/deserialization
- Compatible with OpenAI tool format

### FunctionData Struct
- Located in `client/src/endpoints/completion/completion_response.rs`
- Represents function call data with name and arguments
- Handles JSON argument serialization

## XML Formatting Utilities

### PromptFormat Constants
- Model-specific formatting constants:
  - `ANTHROPIC_PROMPT_FORMAT`
  - `MISTRAL_PROMPT_FORMAT`
  - `LLAMA3_PROMPT_FORMAT`
  - `COMMAND_R_PROMPT_FORMAT`
  - `QWEN_PROMPT_FORMAT`
  - `DEEPSEEK_PROMPT_FORMAT`
- Contains tool call and tool output format specifications
- Reusable for generating consistent XML structures

### Tool XML Generation Logic
- String formatting for tool embedding
- Pre-defined XML structure with opening and closing tags
- Model-specific tool call formatting

## Parsing Components

### Regex Pattern Matching
- XML tag extraction patterns
- Model-specific format handling
- Content extraction logic

### JSON Parsing
- Tool call JSON extraction and parsing
- Error handling for malformed JSON
- Conversion to structured data types

## Error Handling Patterns

### StraicoError
- Error handling for parsing failures
- Network request errors
- JSON serialization/deserialization errors

### Custom Error Types
- Tool embedding errors
- Content merging errors
- Validation errors

## Model Detection Logic

### Model-Specific Format Selection
- Logic to determine prompt format based on model name
- Case-insensitive model name matching
- Fallback to default format

## Content Handling Utilities

### Content Enum Operations
- String conversion methods
- Empty content detection
- Content replacement operations

## Integration Points

### Chat.to_prompt() Method
- Tool embedding integration point
- Message formatting logic
- Content assembly process

### Message.tool_calls_response() Method
- Tool call parsing integration point
- Response processing logic
- Structured data extraction

## Summary

The current implementation has well-encapsulated, reusable components that can be leveraged for the new chat endpoint with minimal modification. The main areas that need adaptation are the embedding location and content merging logic, while preserving the core XML generation and parsing functionality.