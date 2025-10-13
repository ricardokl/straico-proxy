use crate::{error::CustomError, openai_types::OpenAiChatRequest};
use actix_web::{post, web, HttpResponse};
use log::{debug, info};
use straico_client::client::StraicoClient;

#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
    pub debug: bool,
    pub log: bool,
}

#[post("/v1/chat/completions")]
pub async fn openai_chat_completion(
    req: web::Json<OpenAiChatRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, CustomError> {
    let mut openai_request = req.into_inner();

    if data.debug || data.log {
        let request_json = serde_json::to_string_pretty(&openai_request).unwrap();
        if data.debug {
            debug!("\n\n===== Request received (raw): =====\n{request_json}");
        }
        if data.log {
            info!("\n\n===== Request received (raw): =====\n{request_json}");
        }
    }

    let chat_request = openai_request.to_straico_request()?;

    let client = data.client.clone();
    let straico_response = client
        .chat()
        .bearer_auth(&data.key)
        .json(chat_request.clone())
        .send()
        .await?;

    let response_bytes = straico_response.bytes().await?;

    if data.debug || data.log {
        let response_json = serde_json::from_slice::<serde_json::Value>(&response_bytes)
            .and_then(|json| serde_json::to_string_pretty(&json))
            .unwrap_or_else(|_| String::from_utf8_lossy(&response_bytes).to_string());

        if data.debug {
            debug!("\n\n===== Response from Straico (raw): =====\n{response_json}");
        }
        if data.log {
            info!("\n\n===== Response from Straico (raw): =====\n{response_json}");
        }
    }

    let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)?;

    Ok(HttpResponse::Ok().json(response_json))
}
