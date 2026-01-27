# Proxy Architecture

## Overview

The proxy crate provides an Actix-web server that proxies requests to the Straico API while maintaining OpenAI API compatibility.

## Server Bootstrap

Server is initialized in `main.rs` with:
- CLI configuration parsing (host, port, heartbeat character)
- Logger setup with flexi_logger
- StraicoClient configuration (connection pooling, timeouts)
- AppState creation and binding

## AppState

Shared state across worker threads:

```rust
pub struct AppState {
    pub client: StraicoClient,        // Straico API client
    pub key: String,                 // API key
    pub heartbeat_char: HeartbeatChar,  // Streaming heartbeat type
}
```

## HTTP Handlers

### Chat Completion
`server.rs::openai_chat_completion` handles chat completion requests:
- Validates OpenAI request format
- Creates `StraicoProvider` instance
- Dispatches to `handle_chat_completion_async`
- Returns streaming or non-streaming response

### Models Handlers
- `GET /v1/models` - List all models
- `GET /v1/models/{model_id}` - Get single model details

## Provider Implementation

`StraicoProvider` handles all Straico API interactions:

```rust
pub struct StraicoProvider {
    pub client: StraicoClient,
    pub key: String,
    pub heartbeat_char: HeartbeatChar,
}

impl StraicoProvider {
    pub fn send_request(&self, request) -> Result<impl Future, Error>;
    pub fn parse_non_streaming(&self, response) -> impl Future;
    pub fn create_streaming_response(&self, model, future) -> Result<HttpResponse, Error>;
}
```

**Responsibilities:**
- Converts OpenAI ↔ Straico request/response formats
- Emulates streaming with heartbeat keep-alive
- Handles error mapping and status code conversion
- Reads API key from `AppState`

## Request Flow

```
Client Request → Format Validation → Straico API Request
                     ↓                      ↓
              (OpenAI format)      (Straico format)
                     ↓                      ↓
              Response Parsing    ← Streaming/Non-streaming
                     ↓
              OpenAI Format → Client Response
```

## Zero-Cost Async Patterns

Handler uses future combinators for zero-allocation async:

```rust
async fn handle_chat_completion_async(
    provider: &StraicoProvider,
    request: OpenAiChatRequest,
) -> Result<HttpResponse, ProxyError>
```

No generic parameters needed - single concrete type eliminates vtable overhead.

## Key Patterns

- **Zero-alloc async**: Future combinators over async blocks
- **Error mapping**: Centralized upstream error handling
- **Stream composition**: Chain initial → heartbeat → response → done
- **Remote handle pattern**: Split future to control heartbeat termination
- **Clone on AppState**: Thread-safe sharing across workers
