use crate::{
    error::CustomError, providers::StraicoProvider, router::Provider, types::OpenAiChatRequest,
};
use actix_web::{get, post, web, HttpResponse};
use futures::TryStreamExt;
use log::warn;
use straico_client::client::StraicoClient;

#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
}

#[get("/v1/models")]
pub async fn models_handler(data: web::Data<AppState>) -> Result<HttpResponse, CustomError> {
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

    let body_stream = straico_response.bytes_stream().map_err(CustomError::from);
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
) -> Result<HttpResponse, CustomError> {
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

    let body_stream = straico_response.bytes_stream().map_err(CustomError::from);
    Ok(response_builder.streaming(body_stream))
}

#[post("/v1/chat/completions")]
pub async fn openai_chat_completion(
    req: web::Json<OpenAiChatRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, CustomError> {
    let openai_request = req.into_inner();

    let provider = StraicoProvider::new(data.client.clone());
    provider.chat(openai_request, &data.key).await
}

#[post("/v1/chat/completions")]
pub async fn router_chat_completion(
    req: web::Json<OpenAiChatRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, CustomError> {
    let openai_request = req.into_inner();

    // Parse provider from model prefix
    let provider = Provider::from_model(&openai_request.chat_request.model)?;

    // Get API key from environment
    let api_key = std::env::var(provider.env_var_name()).map_err(|_| {
        CustomError::BadRequest(format!(
            "API key not found for provider: {}. Set {} environment variable.",
            provider,
            provider.env_var_name()
        ))
    })?;

    // Handle Straico separately (needs conversion)
    if provider.needs_conversion() {};

    let implementation = provider.get_implementation(&data.client);
    implementation.chat(openai_request, &api_key).await
}
