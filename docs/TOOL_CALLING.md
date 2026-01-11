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

## Provider-Specific Handling

### Straico (Emulated)

**Request:**
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

**Response:**
```json
{
  "role": "assistant",
  "content": "I'll check the weather.\n\n{\"function_calls\": [...]}",
  "tool_calls": [...]
}
```

### Groq/Cerebras/SambaNova (Native)

**Request:**
- Pass `tools` parameter directly
- Pass `tool_choice` parameter directly
- Minimal system message

**Response:**
- Extract `tool_calls` from metadata
- Format is already OpenAI-compatible

## Message Conversions

### OpenAI → Straico (Assistant with Tools)

```rust
match provider {
    Native => {
        ChatMessage::Assistant { content, }  // Tool calls in metadata
    }
    Straico => {
        // Extract tool calls to JSON block
        let combined = format!("{}{}", content, function_calls_json);
        ChatMessage::Assistant { content: combined }
    }
}
```

### Straico → OpenAI (Assistant)

```rust
match provider {
    Native => {
        OpenAiChatMessage::Assistant {
            content: Some(content),
            tool_calls: extract_from_metadata(),
        }
    }
    Straico => {
        // Parse JSON block from content
        let (content_part, tool_calls) = parse_function_calls(content);
        OpenAiChatMessage::Assistant {
            content: content_part,
            tool_calls,
        }
    }
}
```

## System Message Generation

```rust
match provider {
    Native => {
        ChatMessage::System {
            content: "You have access to tools. Use tool_calls when needed."
        }
    }
    Straico => {
        ChatMessage::System {
            content: format!(r#"
You have access to tools: {tools_json}

Respond with function calls in JSON format:
{{"function_calls": [{{"name": "...", "arguments": {{...}}}}]}}"#
            )
        }
    }
}
```

## Request Flow

1. Client sends OpenAI request with `tools` and `tool_choice`
2. Proxy detects provider from model
3. **Native providers**: Pass through directly
4. **Straico**: Inject tools into system message, add JSON instructions
5. Upstream returns response
6. Proxy converts to OpenAI format:
   - Native: Extract `tool_calls` from metadata
   - Straico: Parse JSON block from content
7. Return response with `tool_calls` array

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

### Straico Emulation

- No parallel tool calls
- `tool_choice` parameter ignored (handled via system prompt)
- Parsing relies on JSON block detection (fragile)
- Tool results must be manually added as user messages

### Native Providers

- Full tool calling support
- Parallel calls supported (varies by provider)
- `tool_choice` fully supported

## Usage Example

```bash
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama-3.3-70b-versatile",
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

## Best Practices

1. Always validate tool parameters (JSON Schema)
2. Handle tool errors gracefully
3. Provide clear tool descriptions
4. Use `tool_choice: "auto"` to let model decide
5. Include tool results as user messages for context
6. Check `finish_reason` for `"tool_calls"` to trigger tool execution
7. Use tool call IDs when submitting results
