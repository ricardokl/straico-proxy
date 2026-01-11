# Streaming Implementation

## SSE Format

Server-Sent Events format:

```
data: <json-payload>\n\n
```

Example:
```
data: {"id":"chatcmpl-abc","object":"chat.completion.chunk",...}\n\n
data: [DONE]\n\n
```

## Core Types

```rust
pub enum SseChunk {
    Data(CompletionStream),    // Regular chunk
    Done(String),             // "[DONE]" terminator
    Error(Value),             // Error information
}

pub struct CompletionStream {
    pub choices: Vec<ChoiceStream>,
    pub object: Box<str>,
    pub id: Box<str>,
    pub model: Box<str>,
    pub created: u64,
    pub usage: Usage,
}

pub struct ChoiceStream {
    pub index: u8,
    pub delta: Delta,            // Incremental data
    pub finish_reason: Option<Box<str>>,
}

pub struct Delta {
    pub role: Option<Box<str>>,
    pub content: Option<Box<str>>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

## Emulated Streaming (Straico)

Straico doesn't support native streaming, so we emulate it:

### Stream Composition

```rust
let response_stream = initial_chunk    // Role: "assistant"
    .chain(heartbeat)               // Every 3 seconds
    .chain(straico_response)         // Full response when ready
    .chain(done);                    // [DONE]
```

### Timeline

```
0s   → Initial chunk (role: "assistant", content: null)
3s   → Heartbeat (empty or invisible char)
6s   → Heartbeat
9s   → Heartbeat
10s  → Actual response (full content)
10s+ → [DONE]
```

## Native Streaming (Generic Providers)

Groq, Cerebras, SambaNova support native streaming:

```rust
let stream = future_response
    .map_ok(|resp| resp.bytes_stream())
    .try_flatten_stream();  // Pass through upstream SSE
```

**Key difference:**
- No heartbeat (upstream streams naturally)
- Direct byte passthrough
- True incremental streaming

## Heartbeat Configuration

```rust
pub enum HeartbeatChar {
    Empty,   // No content (default)
    Zwsp,     // Zero-width space (\u200b)
    Zwnj,     // Zero-width non-joiner (\u200c)
    Wj,       // Word joiner (\u2060)
}
```

CLI usage:
```bash
straico-proxy --heartbeat-char empty    # Default
straico-proxy --heartbeat-char zwsp
straico-proxy --heartbeat-char zwnj
```

## SSE Serialization

Convert `SseChunk` to bytes:

```rust
let mut bytes = Vec::with_capacity(json_len + 8);
bytes.extend_from_slice(b"data: ");
bytes.extend_from_slice(&json_bytes);
bytes.extend_from_slice(b"\n\n");
```

## Error Handling in Streams

Map errors to SSE error chunks:

```rust
.stream.map(|result| match result {
    Ok(chunk) => SseChunk::from(chunk).try_into(),
    Err(e) => SseChunk::from(e).try_into(),  // Error chunk
})
```

Error chunk format:
```json
data: {"error":{"message":"...","type":"...","code":"..."}}\n\n
```

## Key Differences

| Aspect | Straico (Emulated) | Generic (Native) |
|---------|---------------------|------------------|
| Source | Single non-streaming response | Upstream SSE stream |
| Heartbeat | Every 3 seconds until response | None |
| Latency | Higher (waits for full response) | Lower (true streaming) |
| Format conversion | Yes (Straico → OpenAI) | No (pass-through) |

## Memory Usage

- **Emulated**: Holds full response in memory
- **Native**: Processes incrementally (~8KB buffer)
