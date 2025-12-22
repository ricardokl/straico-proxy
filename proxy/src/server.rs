use crate::{
    error::ProxyError,
    router::Provider,
    streaming::{CompletionStream, SseChunk},
    types::OpenAiChatRequest,
};
use actix_web::{get, post, web, HttpResponse};
use futures::{future, stream, FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use log::{debug, warn};
use std::time::{SystemTime, UNIX_EPOCH};
use straico_client::client::StraicoClient;
use straico_client::{OpenAiChatResponse, StraicoChatRequest, StraicoChatResponse};
use tokio::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
    pub router_client: Option<reqwest::Client>,
}

#[get("/v1/models")]
pub async fn models_handler(data: web::Data<AppState>) -> Result<HttpResponse, ProxyError> {
    let client = data.client.clone();
    let straico_response = client.models().bearer_auth(&data.key).send().await?;

    let status_code = actix_web::http::StatusCode::from_u16(straico_response.status().as_u16())
        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

    let mut response_builder = HttpResponse::build(status_code);

    // Copy headers from the Straico response to the new response
    for (name, value) in straico_response.headers().iter() {
        if let Ok(value_str) = value.to_str() {
            response_builder.insert_header((name.as_str(), value_str));
        } else {
            warn!("Skipping header with non-ASCII value: {:?}", name);
        }
    }

    let body_stream = straico_response.bytes_stream().map_err(ProxyError::from);
    Ok(response_builder.streaming(body_stream))
}

/// Proxies a request for a single model to Straico's `GET /v2/models/{model_id}` endpoint.
///
/// This mirrors OpenAI's `GET /v1/models/{model}` endpoint. The `{model_id}` path
/// parameter may contain slashes (e.g. `amazon/nova-lite-v1`), so we capture the
/// entire remaining path segment.
#[get("/v1/models/{model_id:.*}")]
pub async fn model_handler(
    data: web::Data<AppState>,
    model_id: web::Path<String>,
) -> Result<HttpResponse, ProxyError> {
    let client = data.client.clone();
    let straico_response = client
        .model(&model_id)
        .bearer_auth(&data.key)
        .send()
        .await?;

    let status_code = actix_web::http::StatusCode::from_u16(straico_response.status().as_u16())
        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

    let mut response_builder = HttpResponse::build(status_code);

    // Copy headers from the Straico response to the new response
    for (name, value) in straico_response.headers().iter() {
        if let Ok(value_str) = value.to_str() {
            response_builder.insert_header((name.as_str(), value_str));
        } else {
            warn!("Skipping header with non-ASCII value: {:?}", name);
        }
    }

    let body_stream = straico_response.bytes_stream().map_err(ProxyError::from);
    Ok(response_builder.streaming(body_stream))
}

#[post("/v1/chat/completions")]
pub async fn openai_chat_completion(
    req: web::Json<OpenAiChatRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, ProxyError> {
    let openai_request = req.into_inner();
    debug!("{}", serde_json::to_string_pretty(&openai_request.clone())?);
    let model = openai_request.chat_request.model.clone();
    let stream = openai_request.stream;
    let AppState {
        ref client,
        ref key,
        ref router_client,
    } = &*data.into_inner();

    // Determine the provider and create the appropriate future
    if let Some(ref router_client) = router_client {
        // Router mode is active
        let provider = Provider::from_model(&model)?;

        if provider == Provider::Straico {
            // Use Straico client even in router mode
            let chat_request = StraicoChatRequest::try_from(openai_request)?;
            let response = client
                .clone()
                .chat()
                .bearer_auth(key)
                .json(chat_request)
                .send();

            if stream {
                Ok(create_streaming_response(&model, response, Some(provider)))
            } else {
                handle_non_streaming_response(response, Some(provider)).await
            }
        } else {
            // Use generic provider with reqwest_client
            let api_key = std::env::var(provider.env_var_name()).map_err(|_| {
                ProxyError::ServerConfiguration(format!(
                    "API key not found for provider: {}. Set {} environment variable.",
                    provider,
                    provider.env_var_name()
                ))
            })?;

            let response = router_client
                .post(provider.base_url())
                .bearer_auth(api_key)
                .json(&openai_request)
                .send();

            if stream {
                Ok(create_streaming_response(&model, response, Some(provider)))
            } else {
                handle_non_streaming_response(response, Some(provider)).await
            }
        }
    } else {
        // Normal mode - always use Straico
        let chat_request = StraicoChatRequest::try_from(openai_request)?;
        let response = client
            .clone()
            .chat()
            .bearer_auth(key)
            .json(chat_request)
            .send();

        if stream {
            Ok(create_streaming_response(&model, response, None))
        } else {
            handle_non_streaming_response(response, None).await
        }
    }
}

/// Safely gets the current Unix timestamp, with fallback for edge cases
fn get_current_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 1704067200, // Fallback to 2024-01-01
    }
}

fn create_streaming_response(
    model: &str,
    future_response: impl future::Future<Output = Result<reqwest::Response, reqwest::Error>>
        + Send
        + 'static,
    provider: Option<Provider>,
) -> HttpResponse {
    match provider {
        Some(Provider::Straico) | None => {
            let id = format!("chatcmpl-{}", Uuid::new_v4());
            let created = get_current_timestamp();

            let initial_chunk = stream::once(future::ready(
                SseChunk::from(CompletionStream::initial_chunk(model, &id, created)).try_into(),
            ));

            let (remote, remote_handle) = future_response.remote_handle();

            let heartbeat = tokio_stream::StreamExt::throttle(
                stream::repeat_with(|| {
                    SseChunk::from(CompletionStream::heartbeat_chunk()).try_into()
                }),
                Duration::from_secs(3),
            )
            .take_until(remote);

            let straico_stream = remote_handle
                .and_then(reqwest::Response::json::<StraicoChatResponse>)
                .map_ok(|x| CompletionStream::try_from(x).unwrap())
                .map_ok(SseChunk::from)
                .map(|result| match result {
                    Ok(chunk) => chunk.try_into(),
                    Err(e) => SseChunk::from(ProxyError::from(e)).try_into(),
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
        _ => {
            let stream = future_response
                .map_ok(|resp| resp.bytes_stream().map_err(ProxyError::from))
                .map_err(ProxyError::from)
                .try_flatten_stream();

            HttpResponse::Ok().streaming(stream)
        }
    }
}

async fn handle_non_streaming_response(
    future_response: impl future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    provider: Option<Provider>,
) -> Result<HttpResponse, ProxyError> {
    let response = future_response.await?;

    let status = response.status();

    // Map upstream 429 responses into a structured rate-limit error
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let provider_name = match provider {
            Some(p) => p.to_string(),
            None => "straico".to_string(),
        };

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
        let provider_name = match provider {
            Some(p) => p.to_string(),
            None => "straico".to_string(),
        };

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

    match provider {
        Some(Provider::Straico) | None => {
            let straico_response: StraicoChatResponse = response.json().await?;
            let openai_response = OpenAiChatResponse::try_from(straico_response)?;
            Ok(HttpResponse::Ok().json(openai_response))
        }
        _ => {
            let json_response: serde_json::Value = response.json().await?;
            Ok(HttpResponse::Ok().json(json_response))
        }
    }
}
