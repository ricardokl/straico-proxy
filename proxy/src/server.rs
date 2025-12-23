use crate::{
    error::ProxyError,
    provider::{ChatProvider, GenericProvider, StraicoProvider},
    router::Provider,
    types::OpenAiChatRequest,
};
use actix_web::{get, post, web, HttpResponse};
use futures::TryStreamExt;
use log::{debug, warn};
use straico_client::client::StraicoClient;

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

/// Generic handler for chat completions that works with any provider implementing ChatProvider.
/// The compiler will monomorphize this function for each concrete provider type, generating
/// specialized code with zero abstraction overhead.
async fn handle_chat_completion_async<P: ChatProvider>(
    provider: &P,
    openai_request: &OpenAiChatRequest,
    model: String,
    stream: bool,
) -> Result<HttpResponse, ProxyError> {
    let response_future = provider.send_request(openai_request)?;

    if stream {
        Ok(provider.create_streaming_response(&model, response_future))
    } else {
        let response = response_future.await?;
        let json = provider.parse_non_streaming(response).await?;
        Ok(HttpResponse::Ok().json(json))
    }
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

    // Determine provider type based on model and router configuration
    let provider_type = if router_client.is_some() {
        // Router mode is active - resolve based on model prefix
        Provider::from_model(&model)?
    } else {
        // Normal mode - always use Straico regardless of model prefix
        Provider::Straico
    };

    // Dispatch to the appropriate monomorphized function based on provider type
    match provider_type {
        Provider::Straico => {
            let provider = StraicoProvider {
                client: client.clone(),
                key: key.clone(),
            };
            handle_chat_completion_async(&provider, &openai_request, model, stream).await
        }
        Provider::Generic(gen_type) => {
            let client = router_client
                .as_ref()
                .ok_or_else(|| {
                    ProxyError::ServerConfiguration(
                        "Router client is not configured for generic provider".to_string(),
                    )
                })?
                .clone();
            let provider = GenericProvider::new(gen_type, client)?;
            handle_chat_completion_async(&provider, &openai_request, model, stream).await
        }
    }
}
