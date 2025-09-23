use crate::{
    config::ProxyConfig, error::CustomError, openai_types::OpenAiChatRequest,
    response_utils::chat_response_utils, tool_embedding::embed_tools_in_chat_request,
};
use actix_web::{post, web, HttpResponse};
use log::debug;
use straico_client::client::StraicoClient;

#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
    pub config: ProxyConfig,
    pub print_request_raw: bool,
    pub print_request_converted: bool,
    pub print_response_raw: bool,
    pub print_response_converted: bool,
}

#[post("/v1/chat/completions")]
pub async fn openai_chat_completion(
    req: web::Json<OpenAiChatRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, CustomError> {
    let openai_request = req.into_inner();

    if data.print_request_raw {
        debug!("\n\n===== Request received (raw): =====");
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&openai_request).unwrap()
        );
    }

    let chat_request = if openai_request.tools.is_some() {
        embed_tools_in_chat_request(openai_request.clone())?
    } else {
        openai_request.to_straico_request()?
    };

    if data.print_request_converted {
        debug!("\n\n===== Request converted (new chat): =====");
        debug!("\n{}", serde_json::to_string_pretty(&chat_request).unwrap());
    }

    let client = data.client.clone();
    let chat_response_data = client
        .chat()
        .bearer_auth(&data.key)
        .json(chat_request.clone())
        .send()
        .await?
        .get_completion()?;

    let chat_response = chat_response_data.get_chat_completion()?;

    let enhanced_response = chat_response_utils::enhance_chat_response(
        chat_response,
        &openai_request,
        data.config.include_debug_info,
    );

    if data.print_response_converted {
        debug!("\n\n===== Response converted: =====");
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&enhanced_response).unwrap()
        );
    }

    Ok(HttpResponse::Ok().json(enhanced_response))
}
