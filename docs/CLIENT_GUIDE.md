# Client Library Guide

## Overview

The `client` crate provides a type-safe Rust client for Straico API with OpenAI format compatibility.

## Client Creation

### Default Client
```rust
let client = StraicoClient::new();
```

### Custom Configuration
```rust
let client = StraicoClient::builder()
    .pool_max_idle_per_host(25)
    .pool_idle_timeout(Duration::from_secs(90))
    .tcp_keepalive(Duration::from_secs(90))
    .timeout(Duration::from_secs(90))
    .build()?;
```

## Request Builder Pattern

### Type-State Builder

Compile-time guarantees for request construction:

```rust
// NoApiKey → ApiKeySet → PayloadSet → Send
client.chat()
    .bearer_auth(&api_key)      // Required
    .json(chat_request)             // Required (POST)
    .send()                        // Execute
    .await
```

### ChatRequestBuilder

Fluent builder for Straico chat requests:

```rust
let request = ChatRequest::<ChatMessage>::builder()
    .model("llama-3.1-70b")
    .message(ChatMessage::user("Hello!"))
    .message(ChatMessage::system("You are helpful."))
    .temperature(0.7)
    .max_tokens(1000)
    .build();
```

## Core Types

### ChatRequest<T>
Generic request structure:
- `model`: Model identifier
- `messages`: Vector of messages
- `temperature`: Optional (0.0-2.0)
- `max_tokens`: Optional

Type aliases:
- `StraicoChatRequest` = `ChatRequest<ChatMessage>`

### OpenAiChatRequest
OpenAI-compatible with streaming and tools:
- Wraps `ChatRequest<OpenAiChatMessage>`
- `stream`: Boolean
- `tools`: Optional vector of `OpenAiTool`
- `tool_choice`: Optional `OpenAiToolChoice`

### Message Types

**ChatMessage** (Straico):
- System, User, Assistant (all require content)

**OpenAiChatMessage** (OpenAI):
- System, User (require content)
- Assistant (optional content, optional `tool_calls`)
- Tool (requires content + `tool_call_id`)

### ChatContent
Dual format support:
- `String` - Plain text
- `Array<Vec<ContentObject>>` - Structured content

## Format Conversions

### OpenAI → Straico Request
```rust
let openai_request = OpenAiChatRequest { /* ... */ };
let straico_request: StraicoChatRequest = openai_request.try_into()?;
```

- Converts message types
- Embeds tools in system messages
- Applies provider-specific formatting

### Straico → OpenAI Response
```rust
let straico_response: StraicoChatResponse = response.json().await?;
let openai_response: OpenAiChatResponse = straico_response.try_into()?;
```

- Strips provider-specific metrics (price, words)
- Extracts tool calls from responses
- Converts to OpenAI format

## API Methods

### Chat Completions
```rust
let response = client.chat()
    .bearer_auth(&api_key)
    .json(chat_request)
    .send()
    .await?;
```

### List Models
```rust
let response = client.models()
    .bearer_auth(&api_key)
    .send()
    .await?;
```

### Get Single Model
```rust
let response = client.model("amazon/nova-lite-v1")
    .bearer_auth(&api_key)
    .send()
    .await?;
```

## Error Handling

```rust
use straico_client::StraicoError;

match result {
    Ok(response) => { /* handle success */ }
    Err(StraicoError::Request(e)) => { /* network error */ }
    Err(StraicoError::Api(msg)) => { /* API error */ }
    Err(StraicoError::Serde(e)) => { /* JSON error */ }
}
```

## Best Practices

1. **Reuse clients** - Don't create new client per request
2. **Set timeouts** - Default 90s, adjust for your use case
3. **Use type-state** - Leverage compile-time safety
4. **Handle errors** - Match on `StraicoError` variants
5. **Check usage** - Monitor `usage` field for token consumption
