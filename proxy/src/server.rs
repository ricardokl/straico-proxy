use crate::{
    config::ProxyConfig,
    endpoint_selection::{should_use_new_endpoint, validate_request_for_endpoint},
    error::CustomError,
    openai_types::OpenAiChatRequest,
    request_conversion::convert_chat_to_legacy_request,
    response_utils::completion_response_utils::convert_chat_response_to_completion,
    tool_embedding::embed_tools_in_chat_request,
};
use actix_web::{post, web, HttpResponse};
use straico_client::client::StraicoClient;
use log::debug;
use rand::distributions::Alphanumeric;
use rand::Rng;
use straico_client::endpoints::completion::completion_response::Completion;

#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
    pub config: ProxyConfig,
    pub print_request_raw: bool,
    pub print_request_converted: bool,
    pub print_response_raw: bool,
    pub print_response_converted: bool,
    pub use_new_chat_endpoint: bool,
    pub force_new_endpoint_for_tools: bool,
}

fn generate_request_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[post("/v1/chat/completions")]
pub async fn openai_chat_completion(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, CustomError> {
    let req_inner = req.into_inner();

    if data.print_request_raw {
        debug!("\n\n===== Request received (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&req_inner).unwrap());
    }

    let openai_request: OpenAiChatRequest = serde_json::from_value(req_inner.clone())?;

    let completion_response = if should_use_new_endpoint(&openai_request, &data) {
        handle_new_chat_endpoint(openai_request, data).await?
    } else {
        handle_legacy_completion_endpoint(openai_request, data).await?
    };

    Ok(HttpResponse::Ok().json(completion_response))
}

async fn handle_new_chat_endpoint(
    openai_request: OpenAiChatRequest,
    data: web::Data<AppState>,
) -> Result<Completion, CustomError> {
    validate_request_for_endpoint(&openai_request, true, &data.config)?;

    let chat_request = if openai_request.tools.is_some() {
        embed_tools_in_chat_request(openai_request)?
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

    let completion_response = convert_chat_response_to_completion(
        chat_response,
        &generate_request_id(),
        current_timestamp(),
    );

    if data.print_response_converted {
        debug!("\n\n===== Response converted: =====");
        debug!("\n{}", serde_json::to_string_pretty(&completion_response).unwrap());
    }

    Ok(completion_response)
}

async fn handle_legacy_completion_endpoint(
    openai_request: OpenAiChatRequest,
    data: web::Data<AppState>,
) -> Result<Completion, CustomError> {
    validate_request_for_endpoint(&openai_request, false, &data.config)?;

    let legacy_request = convert_chat_to_legacy_request(openai_request);

    if data.print_request_converted {
        debug!("\n\n===== Request converted (legacy): =====");
        debug!("\n{}", serde_json::to_string_pretty(&legacy_request).unwrap());
    }

    let client = data.client.clone();
    let legacy_response_data = client
        .completion()
        .bearer_auth(&data.key)
        .json(legacy_request)
        .send()
        .await?
        .get_completion()?;

    let completion = legacy_response_data.get_completion_data();

    if data.print_response_converted {
        debug!("\n\n===== Response converted: =====");
        debug!("\n{}", serde_json::to_string_pretty(&completion).unwrap());
    }

    Ok(completion)
}
