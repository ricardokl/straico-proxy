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

Straico doesn't support native streaming, so we emulate it using a future-based pattern:

### Stream Composition

```rust
let (remote, remote_handle) = future_response.remote_handle();

let initial_chunk = stream::once(future::ready(
    SseChunk::from(CompletionStream::initial_chunk(model, &id, created)).try_into(),
));

let heartbeat = tokio_stream::StreamExt::throttle(
    stream::repeat(heartbeat_chunk).map(Ok::<Bytes, ProxyError>),
    Duration::from_secs(3),
)
.take_until(remote);  // Stop when API response arrives

let straico_stream = remote_handle
    .and_then(reqwest::Response::json::<StraicoChatResponse>)
    .map(/* ... */)
    .into_stream();

let response_stream = initial_chunk
    .chain(heartbeat)
    .chain(straico_stream)
    .chain(done);
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

### Key Pattern: Remote Handle

The `remote_handle()` pattern is critical for heartbeat management:

1. **Split the future** - `let (remote, remote_handle) = future_response.remote_handle();`
2. **Heartbeat stream** - Repeats every 3 seconds
3. **Termination signal** - `.take_until(remote)` stops heartbeat when API responds
4. **Response processing** - `remote_handle` processes the actual response

This ensures heartbeats send while waiting, then automatically stop when the response arrives.

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

## Implementation Details

### Heartbeat Configuration

The heartbeat character can be customized via CLI:

```bash
straico-proxy --heartbeat-char empty    # Default (no content)
straico-proxy --heartbeat-char zwsp     # Zero-width space (\u200b)
straico-proxy --heartbeat-char zwnj     # Zero-width non-joiner (\u200c)
straico-proxy --heartbeat-char wj       # Word joiner (\u2060)
```

### Memory Usage

- **Emulated streaming**: Holds full response in memory until ready
- **Heartbeat overhead**: Minimal (single `Bytes` chunk repeated)
- **Stream composition**: Zero-copy chaining via `chain()` combinator

### Performance Characteristics

- **Latency**: Higher than native streaming (waits for full API response)
- **Throughput**: Single response chunk when ready
- **Client experience**: Streaming appearance via heartbeat keep-alive
- **Resource usage**: Minimal (no incremental parsing needed)
