use crate::{
    error::ProxyError,
    router::{GenericProviderType, Provider},
    streaming::{CompletionStream, HeartbeatChar, SseChunk},
    types::{OpenAiChatRequest, OpenAiChatResponse, StraicoChatResponse},
};
use actix_web::HttpResponse;
use futures::{future, stream, FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use std::future::Future;
use std::time::{SystemTime, UNIX_EPOCH};
use straico_client::client::StraicoClient;
use straico_client::StraicoChatRequest;
use tokio::time::Duration;
use uuid::Uuid;

/// Trait encapsulating provider-specific behavior for chat completions.
pub trait ChatProvider {
    /// Logical provider kind (Straico or a specific generic provider).
    fn provider_kind(&self) -> Provider;

    /// Build and send the upstream request.
    fn send_request(
        &self,
        request: &OpenAiChatRequest,
    ) -> Result<impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static, ProxyError>;

    /// Parse a non-streaming response into a JSON value.
    ///
    /// This keeps error mapping centralized while allowing providers to control
    /// how successful bodies are interpreted.
    fn parse_non_streaming(
        &self,
        response: reqwest::Response,
    ) -> impl Future<Output = Result<serde_json::Value, ProxyError>>;

    /// Create a streaming HTTP response from the upstream future.
    fn create_streaming_response(
        &self,
        model: &str,
        response_future: impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static,
    ) -> HttpResponse;
}

/// Provider implementation for the native Straico backend.
#[derive(Clone)]
pub struct StraicoProvider {
    pub client: StraicoClient,
    pub key: String,
    pub heartbeat_char: HeartbeatChar,
}

impl ChatProvider for StraicoProvider {
    fn provider_kind(&self) -> Provider {
        Provider::Straico
    }

    fn send_request(
        &self,
        request: &OpenAiChatRequest,
    ) -> Result<impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static, ProxyError>
    {
        let chat_request = StraicoChatRequest::try_from(request.clone())?;
        Ok(self
            .client
            .clone()
            .chat()
            .bearer_auth(&self.key)
            .json(chat_request)
            .send())
    }

    fn parse_non_streaming(
        &self,
        response: reqwest::Response,
    ) -> impl Future<Output = Result<serde_json::Value, ProxyError>> {
        // Chain the asynchronous operations using future combinators instead of `async/await`.
        // This avoids heap allocation (`Box`) and the `async` keyword.
        map_common_non_streaming_errors(response, None)
            .and_then(|response| {
                // `response.json()` is an asynchronous call, so we chain it with `and_then`.
                // We use `map_err` to convert its `reqwest::Error` into our `ProxyError`
                // to match the error type of the chain.
                response
                    .json::<StraicoChatResponse>()
                    .map_err(ProxyError::from)
            })
            .then(|result| {
                // `.then` is used because we need to perform synchronous operations
                // on the final `Result`. It receives the `Result` directly.
                //
                // The original `async` block used the `?` operator to chain these
                // final, synchronous transformations. We replicate that logic here.
                // The `and_then` on the `Result` type mirrors the `?` operator.
                let final_result = result.and_then(|straico_response| {
                    let openai_response = OpenAiChatResponse::try_from(straico_response)?;
                    serde_json::to_value(openai_response).map_err(ProxyError::from)
                });

                // The `then` combinator requires a `Future` to be returned.
                // Since our transformations were synchronous, we wrap the final `Result`
                // in an immediately-resolved future using `future::ready`.
                future::ready(final_result)
            })
    }

    fn create_streaming_response(
        &self,
        model: &str,
        response_future: impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static,
    ) -> HttpResponse {
        create_straico_streaming_response(model, response_future, self.heartbeat_char)
    }
}

/// Provider implementation for generic router-backed providers.
#[derive(Clone)]
pub struct GenericProvider {
    pub provider: GenericProviderType,
    pub client: reqwest::Client,
    pub api_key: String,
}

impl GenericProvider {
    pub fn new(provider: GenericProviderType, client: reqwest::Client) -> Result<Self, ProxyError> {
        let provider_kind = Provider::Generic(provider);
        let api_key = std::env::var(provider_kind.env_var_name()).map_err(|_| {
            ProxyError::ServerConfiguration(format!(
                "API key not found for provider: {}. Set {} environment variable.",
                provider_kind,
                provider_kind.env_var_name()
            ))
        })?;

        Ok(Self {
            provider,
            client,
            api_key,
        })
    }
}

impl ChatProvider for GenericProvider {
    fn provider_kind(&self) -> Provider {
        Provider::Generic(self.provider)
    }

    fn send_request(
        &self,
        request: &OpenAiChatRequest,
    ) -> Result<impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static, ProxyError>
    {
        let provider = self.provider_kind();

        Ok(self
            .client
            .post(provider.base_url())
            .bearer_auth(&self.api_key)
            .json(request)
            .send())
    }

    fn parse_non_streaming(
        &self,
        response: reqwest::Response,
    ) -> impl Future<Output = Result<serde_json::Value, ProxyError>> {
        let provider = self.provider_kind();

        // Chain the asynchronous operations using combinators to avoid `async` and `Box`.
        // This keeps the implementation zero-alloc and consistent with `StraicoProvider`.
        map_common_non_streaming_errors(response, Some(provider)).and_then(|response| {
            // Chain the next async call, `.json()`.
            // Map its `reqwest::Error` to our `ProxyError` to satisfy the chain.
            response
                .json::<serde_json::Value>()
                .map_err(ProxyError::from)
        })
    }

    fn create_streaming_response(
        &self,
        _model: &str,
        response_future: impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static,
    ) -> HttpResponse {
        create_generic_streaming_response(response_future)
    }
}

/// Safely gets the current Unix timestamp, with fallback for edge cases.
fn get_current_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 1704067200, // Fallback to 2024-01-01
    }
}

fn create_straico_streaming_response(
    model: &str,
    future_response: impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static,
    heartbeat_char: HeartbeatChar,
) -> HttpResponse {
    let id = format!("chatcmpl-{}", Uuid::new_v4());
    let created = get_current_timestamp();

    let initial_chunk = stream::once(future::ready(
        SseChunk::from(CompletionStream::initial_chunk(model, &id, created)).try_into(),
    ));

    let (remote, remote_handle) = future_response.remote_handle();

    let heartbeat = tokio_stream::StreamExt::throttle(
        stream::repeat_with(move || {
            SseChunk::from(CompletionStream::heartbeat_chunk(&heartbeat_char)).try_into()
        }),
        Duration::from_secs(3),
    )
    .take_until(remote);

    let straico_stream = remote_handle
        .and_then(reqwest::Response::json::<StraicoChatResponse>)
        .map(|result| {
            result
                .map_err(ProxyError::from)
                .and_then(CompletionStream::try_from)
        })
        .map_ok(SseChunk::from)
        .map(|result| match result {
            Ok(chunk) => chunk.try_into(),
            Err(e) => SseChunk::from(e).try_into(),
        })
        .into_stream();

    let done = stream::once(future::ready(
        SseChunk::from("[DONE]".to_string()).try_into(),
    ));

    let response_stream = initial_chunk
        .chain(heartbeat)
        .chain(straico_stream)
        .chain(done);

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(response_stream)
}

fn create_generic_streaming_response(
    future_response: impl Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static,
) -> HttpResponse {
    let stream = future_response
        .map_ok(|resp| resp.bytes_stream().map_err(ProxyError::from))
        .map_err(ProxyError::from)
        .try_flatten_stream();

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(stream)
}

async fn map_common_non_streaming_errors(
    response: reqwest::Response,
    provider: Option<Provider>,
) -> Result<reqwest::Response, ProxyError> {
    let status = response.status();

    let provider_name = provider
        .map(|p| p.to_string())
        .unwrap_or_else(|| "straico".to_string());

    // Map upstream 429 responses into a structured rate-limit error
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        return Err(ProxyError::RateLimited {
            retry_after,
            message: format!("Rate limited by {} API", provider_name),
        });
    }

    // Map common upstream error statuses to structured ProxyError variants
    if status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || status == reqwest::StatusCode::NOT_FOUND
        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
    {
        let body = response.text().await.unwrap_or_default();

        let base_message = format!(
            "{} API returned {} {}",
            provider_name,
            status.as_u16(),
            status.canonical_reason().unwrap_or(""),
        );

        let message = if body.is_empty() {
            base_message
        } else {
            format!("{}: {}", base_message, body)
        };

        let error = if status == reqwest::StatusCode::UNAUTHORIZED {
            ProxyError::Unauthorized(message)
        } else if status == reqwest::StatusCode::FORBIDDEN {
            ProxyError::Forbidden(message)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            ProxyError::NotFound(message)
        } else {
            ProxyError::ServiceUnavailable(message)
        };

        return Err(error);
    }

    // Catch-all for other 4xx/5xx errors
    if status.is_client_error() || status.is_server_error() {
        let body = response.text().await.unwrap_or_default();

        let base_message = format!(
            "{} API returned {} {}",
            provider_name,
            status.as_u16(),
            status.canonical_reason().unwrap_or(""),
        );

        let message = if body.is_empty() {
            base_message
        } else {
            format!("{}: {}", base_message, body)
        };

        return Err(ProxyError::UpstreamError(status.as_u16(), message));
    }

    Ok(response)
}
