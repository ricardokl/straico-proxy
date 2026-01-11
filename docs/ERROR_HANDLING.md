# Error Handling

## Error Types

### Client Errors (StraicoError)

```rust
pub enum StraicoError {
    Request(reqwest::Error),
    Serde(serde_json::Error),
    Api(String),
    Regex(regex::Error),
    ResponseParse(String),
}
```

### Proxy Errors (ProxyError)

```rust
pub enum ProxyError {
    SerdeJson(serde_json::Error),
    ReqwestClient(reqwest::Error),
    Straico(StraicoError),
    ResponseParse(Value),
    ToolEmbedding(String),
    MissingRequiredField { field: String },
    InvalidParameter { parameter: String, reason: String },
    Chat(ChatError),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    RateLimited { retry_after: Option<u64>, message: String },
    ServiceUnavailable(String),
    ServerConfiguration(String),
    UpstreamError(u16, String),
}
```

## Automatic Conversion

Using `#[from]` enables `?` operator:

```rust
impl From<ReqwestError> for ProxyError {
    fn from(e: ReqwestError) -> Self {
        ProxyError::ReqwestClient(e)
    }
}

// Usage
let response = client.send().await?;  // Auto-converts errors
```

## HTTP Status Code Mapping

`ProxyError::status_code()` maps errors to HTTP status:

| Error Variant | Status Code |
|---------------|-------------|
| SerdeJson, BadRequest, InvalidParameter | 400 Bad Request |
| Unauthorized | 401 Unauthorized |
| Forbidden | 403 Forbidden |
| NotFound | 404 Not Found |
| RateLimited | 429 Too Many Requests |
| ServiceUnavailable, ServerConfiguration | 503 Service Unavailable |
| UpstreamError(status, _) | Returns upstream status |
| ReqwestClient (timeout) | 504 Gateway Timeout |
| ReqwestClient (connect) | 502 Bad Gateway |

## OpenAI-Compatible Error Format

```rust
impl ResponseError for ProxyError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(json!({
            "error": {
                "message": self.error_message(),
                "type": self.error_type(),
                "code": self.error_code()
            }
        }))
    }
}
```

**Response format:**
```json
{
  "error": {
    "message": "Missing required field: model",
    "type": "invalid_request_error",
    "code": "missing_field"
  }
}
```

## Error Type Classification

| Type | When Used |
|------|-----------|
| `invalid_request_error` | Bad request, missing field, invalid parameter |
| `authentication_error` | Unauthorized (401) |
| `permission_error` | Forbidden (403) |
| `rate_limit_error` | Too many requests (429) |
| `api_error` | Upstream error, network error |
| `server_error` | Internal server error |

## Streaming Error Handling

Convert errors to SSE error chunks:

```rust
impl ProxyError {
    pub fn to_streaming_chunk(&self) -> Value {
        json!({
            "error": {
                "message": self.error_message(),
                "type": self.error_type(),
                "code": self.error_code()
            }
        })
    }
}
```

**SSE error chunk:**
```
data: {"error":{"message":"...","type":"...","code":"..."}}\n\n
```

## Upstream Error Mapping

Centralized error handling for upstream responses:

```rust
async fn map_common_non_streaming_errors(
    response: reqwest::Response,
    provider: Option<Provider>,
) -> Result<Response, ProxyError> {
    let status = response.status();

    if status == TOO_MANY_REQUESTS {
        let retry_after = extract_retry_after(&response);
        return Err(ProxyError::RateLimited {
            retry_after,
            message: format!("Rate limited by {provider} API"),
        });
    }

    if status.is_client_error() || status.is_server_error() {
        return Err(ProxyError::UpstreamError(
            status.as_u16(),
            format!("{provider} API returned {status}")
        ));
    }

    Ok(response)
}
```

## Error Propagation Patterns

### Pattern 1: Direct Propagation

```rust
async fn handle() -> Result<Response, ProxyError> {
    let response = fetch().await?;  // Auto-converts
    let parsed = parse(response).await?;  // Auto-converts
    Ok(HttpResponse::Ok().json(parsed))
}
```

### Pattern 2: Context Addition (top-level)

```rust
use anyhow::Context;

async fn main() -> anyhow::Result<()> {
    let response = client.send()
        .await
        .context("Failed to send request")?;

    let result = response.json().await
        .context("Failed to parse response")?;

    Ok(())
}
```

## Best Practices

1. **Use `thiserror`** - Automatic `Display` and `Error` impls
2. **Implement `From`** - Enable `?` operator
3. **Map HTTP status codes** - Return appropriate codes
4. **Use `ResponseError`** - Automatic HTTP response formatting
5. **Map streaming errors** - Convert to SSE chunks
6. **Handle upstream errors** - Map 4xx/5xx to typed errors
7. **Provide context** - Use `anyhow::Context` for top-level errors
8. **Be specific** - Use descriptive error variants
9. **Preserve upstream info** - Include provider and status in messages
10. **Test error paths** - Verify error responses match expectations
