use crate::{error::ProxyError, types::OpenAiChatRequest};
use actix_web::HttpResponse;
use futures::TryStreamExt;

pub enum ProviderImpl {
    Generic(GenericProvider),
}

impl ProviderImpl {
    pub async fn chat(
        &self,
        request: OpenAiChatRequest,
        api_key: &str,
    ) -> Result<HttpResponse, ProxyError> {
        match self {
            ProviderImpl::Generic(p) => p.chat(request, api_key).await,
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
