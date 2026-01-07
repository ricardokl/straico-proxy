# Agent Guidelines - Straico Proxy

## Project Overview

Rust workspace with two crates:
- `proxy/` - Actix-web proxy server with OpenAI-compatible endpoints
- `client/` - Straico API client library

The proxy enables tool calling, streaming with heartbeat, and multi-provider routing.

## Build & Test Commands

### Build
```bash
# Build all workspace members
cargo build

# Build specific package
cargo build -p straico-proxy
cargo build -p straico-client
```

### Lint
```bash
# Run Clippy
cargo clippy

# Fix Clippy warnings automatically
cargo clippy --fix

# Check without building
cargo check

# Check specific package
cargo clippy -p straico-proxy
```

### Test
```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test -p straico-proxy
cargo test -p straico-client

# Run single test
cargo test test_name_here

# Run tests with output
cargo test -- --nocapture
```

## Code Style Guidelines

### Imports
- Use workspace dependencies from root `Cargo.toml`
- Re-export commonly used types from `types.rs`
- Prefer `use crate::` over absolute paths within crate
- Group imports: std → external → internal

### Formatting
- Uses default `rustfmt` settings (no custom config)
- Run `cargo fmt` before commits

### Types
- Prefer `thiserror` for error enums with automatic `Display` and `Error` impls
- Use `serde` derive for all request/response types
- `Option<T>` for nullable fields with `#[serde(skip_serializing_if = "Option::is_none")]`
- `Box<str>`/`Box<[T]>` for heap-allocated owned data to reduce size
- Use `async-trait` for async trait methods

```rust
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Failed to serialize JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Missing required field: {field}")]
    MissingRequiredField { field: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<str>>,
}
```

### Naming Conventions
- **Structs/Enums**: `PascalCase` - `AppState`, `ChatProvider`, `ProxyError`
- **Functions/Methods**: `snake_case` - `send_request`, `parse_non_streaming`
- **Constants**: `SCREAMING_SNAKE_CASE` - `DEFAULT_TIMEOUT`
- **Type Parameters**: `PascalCase` (single char) or descriptive `PascalCase` - `T`, `Provider`
- **Modules**: `snake_case` - `provider.rs`, `streaming.rs`
- **Private fields**: `snake_case` - `api_key`, `heartbeat_char`

### Error Handling
- Define error enums with `thiserror::Error`
- Implement `From` for foreign errors to enable `?` operator
- Use `anyhow::Context` for adding context to `anyhow::Result` chains
- Map HTTP status codes to appropriate error variants
- Return `ProxyError::UpstreamError(status, message)` for generic upstream errors
- Implement `ResponseError` from actix-web for automatic HTTP responses

```rust
impl From<ReqwestError> for ProxyError {
    fn from(e: ReqwestError) -> Self {
        ProxyError::ReqwestClient(e)
    }
}

async fn map_common_non_streaming_errors(
    response: reqwest::Response,
) -> Result<reqwest::Response, ProxyError> {
    if status.is_client_error() {
        return Err(ProxyError::BadRequest("Invalid request".to_string()));
    }
    Ok(response)
}
```

### Async Patterns
- Use `async fn` for top-level handlers and API calls
- Use `.await` in async contexts, avoid blocking calls
- Prefer future combinators over `async move ||` blocks when possible (zero-alloc)
- Use `future::ready()` for wrapping synchronous results in futures
- Stream processing with `futures::StreamExt` for streaming responses

```rust
// Prefer combinators over async blocks (zero-alloc)
response.json::<T>().map_err(ProxyError::from).then(|result| {
    let final_result = result.and_then(|value| transform(value)?);
    future::ready(final_result)
})

// Use async/await when necessary for clarity
async fn handle_request() -> Result<HttpResponse, ProxyError> {
    let response = fetch().await?;
    Ok(HttpResponse::Ok().json(response))
}
```

### Traits
- Use traits for abstraction across multiple implementations (`ChatProvider`)
- Use `async-trait` for async trait methods
- Implement `From` and `TryFrom` for type conversions between formats
- Implement `Display` for enums/structs that need string representation

```rust
#[async_trait]
pub trait ChatProvider {
    fn provider_kind(&self) -> Provider;
    async fn send_request(&self, request: Request) -> Result<Response, Error>;
}

impl From<StraicoResponse> for OpenAiResponse {
    fn from(value: StraicoResponse) -> Self {
        // Conversion logic
    }
}
```

### Testing
- Unit tests in `#[cfg(test)]` modules at bottom of files
- Test functions: `fn test_<description>()`
- Use `assert!`, `assert_eq!`, `assert_matches!` for assertions
- Test error paths with `assert!(result.is_err())`
- Integration tests in `tests/` directory (if needed)

### HTTP/API Patterns
- Use `actix-web` for HTTP server with extractors (`web::Json`, `web::Data`, `web::Path`)
- Implement `Clone` on `AppState` for thread-safe sharing
- Return `HttpResponse::Ok().json()` for JSON responses
- Use `web::Data<AppState>` for dependency injection
- Stream responses with `HttpResponse::Ok().content_type("text/event-stream").streaming()`

### Constants & Config
- Use `lazy_static` or `once_cell` for static values with initialization
- Default values in CLI args with `#[arg(long, default_value = "...")]`
- Environment variables with `#[arg(long, env = "VAR_NAME")]`

### Router Pattern
- Provider routing via `Provider` enum with `from_model(&str)` parsing
- Model format: `<provider>/<model-name>` (e.g., `groq/llama-3.1-70b`)
- Each provider has `base_url()` and `env_var_name()` methods
- Monomorphized generic functions for zero-cost abstraction

### Streaming
- Server-Sent Events (SSE) format: `data: <json>\n\n`
- Heartbeat chunks every 3 seconds for keep-alive
- Use `tokio_stream::StreamExt::throttle` for timing control
- Map errors to SSE error chunks in streams

## Notes

- No `.rustfmt.toml` - uses default Rust formatting
- No `clippy.toml` - uses default Clippy lints
- Workspace resolver enabled (`resolver = "2"` in root `Cargo.toml`)
- HTTP client uses `rustls-tls` for TLS (no OpenSSL dependency)
