# Tool Calling Support

## Overview

Straico Proxy supports OpenAI-compatible tool calling. Since Straico lacks native tool calling, it's emulated via system messages and JSON parsing.

## Tool Types

```rust
pub struct OpenAiTool {
    pub r#type: String,              // "function"
    pub function: OpenAiFunction,
}

pub struct OpenAiFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

pub struct ToolCall {
    pub id: String,
    pub r#type: String,              // "function"
    pub function: ChatFunctionCall,
}

pub struct ChatFunctionCall {
    pub name: String,
    pub arguments: String,             // JSON string
}

pub enum OpenAiToolChoice {
    Auto,                            // Model decides
    Required,                         // Must call tool
    None,                             // No tools
    Function { name: String },
}
```

## Straico Tool Calling (Emulated)

Since Straico doesn't support native tool calling, it's emulated via system messages and JSON parsing.

### Request Handling

1. Inject tool definitions into system message
2. Include JSON formatting instructions
3. Model writes function calls in JSON block

**System message format:**
```
You have access to following tools:
{tools_json}

When calling a function, respond with:
{"function_calls": [{"name": "func_name", "arguments": {...}}]}
```

### Response Handling

```json
{
  "role": "assistant",
  "content": "I'll check the weather.\n\n{\"function_calls\": [...]}",
  "tool_calls": [...]
}
```

The proxy:
1. Parses the JSON block from the response content
2. Extracts tool calls
3. Returns OpenAI-compatible format with `tool_calls` array

## Message Conversions

### OpenAI → Straico (Assistant with Tools)

When converting OpenAI messages with tool calls to Straico format:

```rust
// Extract tool calls to JSON block
let combined = format!("{}{}", content, function_calls_json);
ChatMessage::Assistant { content: combined }
```

The tool calls are embedded in the content as a JSON block.

### Straico → OpenAI (Assistant)

When converting Straico responses back to OpenAI format:

```rust
// Parse JSON block from content
let (content_part, tool_calls) = parse_function_calls(content);
OpenAiChatMessage::Assistant {
    content: content_part,
    tool_calls,
}
```

The proxy extracts tool calls from the JSON block in the response content.

## System Message Generation

For Straico, the proxy injects tool definitions into the system message:

```rust
ChatMessage::System {
    content: format!(r#"
You have access to tools: {tools_json}

Respond with function calls in JSON format:
{{"function_calls": [{{"name": "...", "arguments": {{...}}}}]}}"#
    )
}
```

## Request Flow

1. Client sends OpenAI request with `tools` and `tool_choice`
2. Proxy validates request format
3. Injects tools into system message with JSON formatting instructions
4. Sends request to Straico API
5. Straico returns response with tool calls in JSON block
6. Proxy parses JSON block and extracts tool calls
7. Returns OpenAI-compatible response with `tool_calls` array

## Tool Message Handling

Straico doesn't support tool messages, so we convert them:

```rust
// OpenAI tool message → Straico user message
OpenAiChatMessage::Tool {
    content,
    tool_call_id,
}
↓
ChatMessage::User {
    content: "Tool output for {tool_call_id}: {content}"
}
```

## Limitations

### Straico Tool Calling

- No parallel tool calls
- `tool_choice` parameter ignored (handled via system prompt)
- Parsing relies on JSON block detection (can be fragile)
- Tool results must be manually added as user messages
- Requires careful prompt engineering for reliable tool calling

## Usage Example

```bash
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "system", "content": "You are a weather assistant."},
      {"role": "user", "content": "What'\''s the weather in NY?"}
    ],
    "tools": [{
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get weather for location",
        "parameters": {
          "type": "object",
          "properties": {
            "location": {"type": "string"}
          },
          "required": ["location"]
        }
      }
    }],
    "tool_choice": "auto"
  }'
```

**Note:** The model name should be a valid Straico model (e.g., `gpt-4o`, `claude-3-5-sonnet`, `llama-3.1-70b`).

## Best Practices

1. Always validate tool parameters (JSON Schema)
2. Handle tool errors gracefully
3. Provide clear tool descriptions
4. Use `tool_choice: "auto"` to let model decide
5. Include tool results as user messages for context
6. Check `finish_reason` for `"tool_calls"` to trigger tool execution
7. Use tool call IDs when submitting results
