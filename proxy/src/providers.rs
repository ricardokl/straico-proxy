use crate::{
    error::ProxyError,
    streaming::{CompletionStream, SseChunk},
    types::{OpenAiChatRequest, OpenAiChatResponse, StraicoChatResponse},
};
use actix_web::HttpResponse;
use futures::{future, stream, FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use log::error;
use std::time::{SystemTime, UNIX_EPOCH};
use straico_client::{client::StraicoClient, StraicoChatRequest};
use tokio::time::Duration;
use uuid::Uuid;

/// Safely gets the current Unix timestamp, with fallback for edge cases
fn get_current_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 1704067200, // Fallback to 2024-01-01
    }
}

pub enum ProviderImpl {
    Straico(StraicoProvider),
    Generic(GenericProvider),
}

impl ProviderImpl {
    pub async fn chat(
        &self,
        request: OpenAiChatRequest,
        api_key: &str,
    ) -> Result<HttpResponse, ProxyError> {
        match self {
            ProviderImpl::Straico(p) => p.chat(request, api_key).await,
            ProviderImpl::Generic(p) => p.chat(request, api_key).await,
        }
    }
}

pub struct StraicoProvider {
    pub client: StraicoClient,
}

impl StraicoProvider {
    pub fn new(client: StraicoClient) -> Self {
        Self { client }
    }

    fn create_streaming_response(
        model: String,
        straico_response: impl future::Future<Output = Result<reqwest::Response, reqwest::Error>>
            + Send
            + 'static,
    ) -> HttpResponse {
        let id = format!("chatcmpl-{}", Uuid::new_v4());
        let created = get_current_timestamp();

        let initial_chunk = stream::once(future::ready(
            SseChunk::from(CompletionStream::initial_chunk(&model, &id, created)).try_into(),
        ));

        let (remote, remote_handle) = straico_response.remote_handle();

        let heartbeat = tokio_stream::StreamExt::throttle(
            stream::repeat_with(|| SseChunk::from(CompletionStream::heartbeat_chunk()).try_into()),
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

    async fn handle_non_streaming_response(
        straico_response: impl future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    ) -> Result<HttpResponse, ProxyError> {
        let straico_response: StraicoChatResponse = straico_response.await?.json().await?;

        let openai_response = match OpenAiChatResponse::try_from(straico_response.clone()) {
            Ok(response) => response,
            Err(e) => {
                error!(
                    "Failed to convert Straico response to OpenAI format: {}\nRaw Straico Response: {}",
                    e,
                    serde_json::to_string_pretty(&straico_response).unwrap_or_default()
                );
                return Err(e.into());
            }
        };
        Ok(HttpResponse::Ok().json(openai_response))
    }

    pub async fn chat(
        &self,
        request: OpenAiChatRequest,
        api_key: &str,
    ) -> Result<HttpResponse, ProxyError> {
        let stream = request.stream;
        let chat_request = StraicoChatRequest::try_from(request)?;
        let model = if stream {
            Some(chat_request.model.clone())
        } else {
            None
        };

        let straico_response = self
            .client
            .clone()
            .chat()
            .bearer_auth(api_key)
            .json(chat_request)
            .send();

        if stream {
            Ok(Self::create_streaming_response(
                model.unwrap(),
                straico_response,
            ))
        } else {
            Self::handle_non_streaming_response(straico_response).await
        }
    }
}

pub struct GenericProvider {
    pub base_url: String,
    pub provider_name: String,
}

impl GenericProvider {
    pub fn new(base_url: String, provider_name: String) -> Self {
        Self {
            base_url,
            provider_name,
        }
    }

    pub async fn chat(
        &self,
        mut request: OpenAiChatRequest,
        api_key: &str,
    ) -> Result<HttpResponse, ProxyError> {
        // Strip the provider prefix from the model name
        if let Some(model_without_prefix) = request.chat_request.model.split('/').nth(1) {
            request.chat_request.model = model_without_prefix.to_string();
        }

        let http_client = reqwest::Client::new();
        let response = http_client
            .post(&self.base_url)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await?;

        let status_code = actix_web::http::StatusCode::from_u16(response.status().as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

        let mut response_builder = HttpResponse::build(status_code);

        // Copy headers from the provider response
        for (name, value) in response.headers().iter() {
            if let Ok(value_str) = value.to_str() {
                response_builder.insert_header((name.as_str(), value_str));
            }
        }

        let body_stream = response.bytes_stream().map_err(ProxyError::from);

        Ok(response_builder.streaming(body_stream))
    }
}
