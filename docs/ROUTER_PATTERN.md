# Straico-Only Architecture

## Overview

As of v0.3.2, the proxy has been simplified to support **Straico only**. The multi-provider router functionality has been removed to reduce complexity while maintaining full Straico support.

## Supported Models

All Straico models are supported. Examples:

- `llama-3.1-70b`
- `gpt-4o`
- `claude-3-5-sonnet`
- `amazon/nova-lite-v1`
- Any model available via Straico API

## Provider Implementation

The proxy uses a single `StraicoProvider` implementation:

```rust
pub struct StraicoProvider {
    pub client: StraicoClient,
    pub key: String,
    pub heartbeat_char: HeartbeatChar,
}

impl StraicoProvider {
    pub fn send_request(&self, request: OpenAiChatRequest)
        -> Result<impl Future, ProxyError>;

    pub fn parse_non_streaming(&self, response: reqwest::Response)
        -> impl Future<Output = Result<serde_json::Value, ProxyError>>;

    pub fn create_streaming_response(
        &self,
        model: &str,
        response_future: impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static,
    ) -> Result<HttpResponse, ProxyError>;
}
```

## Request Flow

```
Client Request (OpenAI format)
        ↓
Format Conversion (OpenAI → Straico)
        ↓
Straico API Request
        ↓
Response Processing
        ↓
Format Conversion (Straico → OpenAI)
        ↓
Client Response (OpenAI format)
```

## Configuration

### API Key

Set via CLI or environment variable:

```bash
# Via CLI
straico-proxy --api-key "your-key"

# Via environment
export STRAICO_API_KEY="your-key"
straico-proxy
```

### Heartbeat Configuration

Control streaming heartbeat behavior:

```bash
straico-proxy --heartbeat-char empty    # Default (no content)
straico-proxy --heartbeat-char zwsp     # Zero-width space
straico-proxy --heartbeat-char zwnj     # Zero-width non-joiner
straico-proxy --heartbeat-char wj       # Word joiner
```

## Streaming Behavior

Straico doesn't support native streaming, so the proxy emulates it:

1. **Initial chunk** - Sent immediately with role "assistant"
2. **Heartbeat** - Sent every 3 seconds while waiting for response
3. **Response chunk** - Full response when Straico API completes
4. **Done marker** - `[DONE]` to signal stream end

This provides a streaming experience while waiting for the API response.

## Error Handling

Errors are mapped to OpenAI-compatible format:

```json
{
  "error": {
    "message": "Rate limited by Straico API",
    "type": "rate_limit_error",
    "code": "rate_limit_exceeded"
  }
}
```

Common error mappings:
- 429 → `rate_limit_error`
- 401/403 → `authentication_error` / `permission_error`
- 4xx/5xx → `api_error`

## Future Enhancements

To add multi-provider support in the future:

1. Restore `ChatProvider` trait
2. Implement `GenericProvider` for other APIs
3. Add provider detection logic
4. Update request routing

See git history for previous multi-provider implementation.
