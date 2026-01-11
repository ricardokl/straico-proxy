# Proxy Architecture

## Overview

The proxy crate provides an Actix-web server that routes requests to multiple upstream providers while maintaining OpenAI API compatibility.

## Server Bootstrap

Server is initialized in `main.rs` with:
- CLI configuration parsing (host, port, router mode, heartbeat)
- Logger setup with flexi_logger
- StraicoClient configuration (connection pooling, timeouts)
- AppState creation and binding

## AppState

Shared state across worker threads:

```rust
pub struct AppState {
    pub client: StraicoClient,        // Straico API client
    pub key: String,                 // API key
    pub router_client: Option<reqwest::Client>,  // Generic provider client
    pub heartbeat_char: HeartbeatChar,  // Streaming heartbeat type
}
```

## HTTP Handlers

### Chat Completion
`server.rs::openai_chat_completion` routes requests based on model prefix:
- Parse provider from model if router enabled
- Match provider to create `StraicoProvider` or `GenericProvider`
- Dispatch to monomorphized `handle_chat_completion_async`

### Models Handlers
- `GET /v1/models` - List all models
- `GET /v1/models/{model_id}` - Get single model details

## Provider Trait

Core abstraction for multi-provider support:

```rust
#[async_trait]
pub trait ChatProvider {
    fn provider_kind(&self) -> Provider;
    fn send_request(&self, request) -> Result<impl Future, Error>;
    fn parse_non_streaming(&self, response) -> impl Future;
    fn create_streaming_response(&self, model, future) -> Result<HttpResponse, Error>;
}
```

### StraicoProvider
- Uses `StraicoClient` for requests
- Converts OpenAI ↔ Straico formats
- Emulates streaming with heartbeat
- Reads API key from `AppState`

### GenericProvider
- Uses `reqwest::Client` directly
- Passes OpenAI format through (no conversion)
- Streams upstream SSE directly
- Reads API key from provider-specific env vars

## Request Flow

```
Client Request → Provider Detection → Format Conversion → Upstream API
                     ↓                      ↓
              (Router or Straico)    (Provider Trait)
                     ↓                      ↓
              Response Parsing    ← Streaming/NON
                     ↓
              OpenAI Format → Client Response
```

## Monomorphization

Generic handler enables zero-cost abstraction:

```rust
async fn handle_chat_completion_async<P: ChatProvider>(
    provider: &P,
    request: OpenAiChatRequest,
) -> Result<HttpResponse, ProxyError>
```

Compiler generates specialized versions for each provider type, eliminating runtime dispatch.

## Key Patterns

- **Zero-alloc async**: Future combinators over async blocks
- **Error mapping**: Centralized upstream error handling
- **Stream composition**: Chain initial → heartbeat → response → done
- **Clone on AppState**: Thread-safe sharing across workers
