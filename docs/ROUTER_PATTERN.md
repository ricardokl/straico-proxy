# Router Pattern

## Model Format

Models use `<provider>/<model-name>` format:

- `groq/llama-3.1-70b` → Groq API
- `cerebras/llama3.3` → Cerebras API
- `sambanova/Meta-Llama-3.1` → SambaNova API
- `llama-3.1-70b` → Straico (no prefix, default)

## Provider Types

```rust
pub enum Provider {
    Straico,
    Generic(GenericProviderType),
}

pub enum GenericProviderType {
    SambaNova,
    Cerebras,
    Groq,
}
```

## Parsing Logic

`Provider::from_model(model)` extracts prefix:

```rust
let prefix = model.split('/').next()?;
match prefix.to_lowercase().as_str() {
    "groq" => Ok(Provider::Generic(Groq)),
    "cerebras" => Ok(Provider::Generic(Cerebras)),
    "sambanova" => Ok(Provider::Generic(SambaNova)),
    "straico" => Ok(Provider::Straico),
    _ => Err("Unknown provider"),
}
```

## Provider Metadata

### Base URLs

| Provider | Base URL |
|----------|-----------|
| Straico | `https://api.straico.com/v2` |
| Groq | `https://api.groq.com/openai/v1/chat/completions` |
| Cerebras | `https://api.cerebras.ai/v1/chat/completions` |
| SambaNova | `https://api.sambanova.ai/v1/chat/completions` |

### Environment Variables

| Provider | Env Var |
|----------|----------|
| Straico | `STRAICO_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Cerebras | `CEREBRAS_API_KEY` |
| SambaNova | `SAMBANOVA_API_KEY` |

## Request Routing

### Router Mode

Enabled via `--router` flag:

```rust
let provider_type = if router_client.is_some() {
    Provider::from_model(model_str)?    // Parse from prefix
} else {
    Provider::Straico                    // Always Straico
};

match provider_type {
    Provider::Straico => handle_with_straico(request).await,
    Provider::Generic(gen_type) => handle_with_generic(gen_type, request).await,
}
```

### Provider Dispatch

**StraicoProvider:**
- Uses `StraicoClient`
- Converts OpenAI ↔ Straico formats
- Emulates streaming with heartbeat
- Reads API key from `AppState.key`

**GenericProvider:**
- Uses `reqwest::Client`
- Passes OpenAI format through
- Streams upstream SSE directly
- Reads API key from provider-specific env var

## Adding a New Provider

1. Add variant to `GenericProviderType`
2. Add `base_url()` match arm
3. Add `env_var_name()` match arm
4. Update `from_model()` parser

Example:
```rust
// Add variant
pub enum GenericProviderType {
    // ...
    NewProvider,
}

// Add base URL
impl GenericProviderType {
    pub fn base_url(&self) -> &'static str {
        match self {
            // ...
            NewProvider => "https://api.new.com/v1/chat/completions",
        }
    }
}

// Add env var
impl Provider {
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Provider::Generic(p) => match p {
                // ...
                NewProvider => "NEW_PROVIDER_API_KEY",
            },
        }
    }
}
```

## Design Rationale

### Why Model Prefix Pattern?

- **Explicit**: Provider visible in model name
- **Simple**: No separate routing config
- **OpenAI-compatible**: Many clients already use this format
- **Stateless**: Request-based routing

### Why Separate Clients?

- **StraicoClient**: Type-safe, format conversions, tool calling
- **reqwest::Client**: Generic, pass-through, supports streaming
