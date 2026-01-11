# Agent Guidelines - Straico Proxy

## Architecture Overview

Rust workspace with two crates:

### `proxy/` - Actix-web proxy server
- **Purpose**: OpenAI-compatible endpoints with multi-provider routing
- **Key Libraries**: Actix-web, tokio, reqwest, futures
- **Main Components**:
  - Provider routing (Straico, SambaNova, Cerebras, Groq)
  - SSE streaming with heartbeat keep-alive
  - Tool calling format conversions
  - Type state pattern for configuration

### `client/` - Straico API client
- **Purpose**: Type-safe client with OpenAI ↔ Straico format conversions
- **Key Libraries**: reqwest, serde, thiserror, regex
- **Main Components**:
  - Builder pattern for API requests
  - Type-safe request/response types
  - Format bridging (OpenAI ↔ Straico)
  - Tool calling emulation support

## Quick Reference

### Build & Test
```bash
# Build
cargo build

# Lint
cargo clippy

# Test
cargo test

# Format
cargo fmt
```

### Module Quick-Links

**Proxy Crate** (`proxy/src/`)
- `main.rs` - Server entry point, CLI setup
- `server.rs` - HTTP handlers, AppState
- `provider.rs` - Provider implementations, `ChatProvider` trait
- `router.rs` - Provider routing, model parsing
- `streaming.rs` - SSE streaming, heartbeat
- `types.rs` - OpenAI type re-exports
- `error.rs` - Proxy error handling
- `cli.rs` - CLI configuration

**Client Crate** (`client/src/`)
- `client.rs` - HTTP client, builder pattern
- `endpoints/chat/` - Chat completions, messages, conversions
- `endpoints/chat/tool_calling/` - Tool calling support
- `endpoints/models/` - Model listing
- `error.rs` - Client error types

## Documentation Files

Detailed documentation is in the `docs/` folder:

| File | Topics Covered |
|------|--------------|
| `PROXY_ARCHITECTURE.md` | Server setup, provider trait, request flow |
| `CLIENT_GUIDE.md` | Client usage, builder patterns, type system |
| `ASYNC_PATTERNS.md` | Future combinators, streaming, monomorphization |
| `ROUTER_PATTERN.md` | Provider detection, model parsing, dispatch |
| `STREAMING.md` | SSE format, heartbeat, chunk composition |
| `TOOL_CALLING.md` | Tool format, system messages, conversions |
| `ERROR_HANDLING.md` | Error enums, status mapping, HTTP responses |

## Key Architectural Patterns

1. **Provider Abstraction** - `ChatProvider` trait for multi-provider routing
2. **Type State Pattern** - Client builder: `NoApiKey` → `ApiKeySet`
3. **Zero-Alloc Async** - Future combinators over `async move` blocks
4. **Format Bridging** - Bidirectional OpenAI ↔ Straico conversions

## Code Style Notes

- Use `thiserror` for error enums
- `Box<str>`/`Box<[T]>` for heap-allocated data
- `Option<T>` with `#[serde(skip_serializing_if)]` for nullable fields
- Monomorphized functions for zero-cost abstraction
- `async-trait` for async trait methods

## Configuration

- No `.rustfmt.toml` - uses default formatting
- No `clippy.toml` - uses default lints
- Workspace resolver enabled (`resolver = "2"`)
- HTTP client uses `rustls-tls` (no OpenSSL)
