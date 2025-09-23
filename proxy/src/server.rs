use crate::{
    content_conversion::convert_openai_request_to_straico, openai_types::OpenAiChatRequest,
};
use crate::{error::CustomError, AppState};
use actix_web::{post, web};
use anyhow::anyhow;
use log::debug;
use rand::distributions::Alphanumeric;
use rand::Rng;
use straico_client::endpoints::chat::ChatResponse;

fn generate_request_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

/// Handles OpenAI-style chat completion requests using the new chat endpoint
///
/// This endpoint processes chat completion requests using the new Straico chat format
/// with proper content format conversion from OpenAI to Straico format.
///
/// # Arguments
/// * `req` - The incoming chat completion request in OpenAI format
/// * `data` - Shared application state containing client and configuration
///
/// # Returns
/// * `Result<impl Responder, Error>` - The completion response or error
#[post("/v1/chat/completions")]
async fn openai_chat_completion(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<web::Json<ChatResponse>, CustomError> {
    let req_inner = req.into_inner();

    if data.print_request_raw {
        debug!("\n\n===== Chat Request received (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&req_inner).unwrap());
    }

    // Parse as OpenAI chat request
    let openai_request: OpenAiChatRequest =
        serde_json::from_value(req_inner.clone()).map_err(|e| {
            CustomError::Anyhow(anyhow::anyhow!("Failed to parse OpenAI request: {}", e))
        })?;

    // Validate request against configuration
    if let Err(validation_error) = data.config.validate_chat_request(&openai_request) {
        return Err(CustomError::Anyhow(anyhow::anyhow!(
            "Request validation failed: {}",
            validation_error
        )));
    }

    // Stash tool_choice and tools to transfer to response
    let tool_choice = openai_request.tool_choice.clone();
    let tools = openai_request.tools.clone();

    // Convert to Straico chat request
    let straico_request = convert_openai_request_to_straico(openai_request.clone())
        .map_err(|e| CustomError::Anyhow(anyhow::anyhow!("Content conversion failed: {}", e)))?;

    if data.print_request_converted {
        debug!("\n\n===== Chat Request converted to Straico: =====");
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&straico_request).unwrap()
        );
    }

    // Make request to new Straico chat endpoint
    let client = data.client.clone();
    let response = client
        .chat()
        .bearer_auth(&data.key)
        .json(straico_request)
        .send()
        .await?;

    if data.print_response_raw {
        debug!("\n\n===== Chat Response received (raw): =====");
        debug!("\n{}", serde_json::to_string_pretty(&response).unwrap());
    }

    // Parse the response from the new chat endpoint
    let mut chat_response: ChatResponse = serde_json::from_value(response.data).map_err(|e| {
        CustomError::Anyhow(anyhow::anyhow!("Failed to parse chat response: {}", e))
    })?;

    // Add tool_choice to the response
    if let Some(tool_choice) = tool_choice {
        if let Some(tools) = &openai_request.tools {
            if !tools.is_null() {
                chat_response.tool_choice = Some(tool_choice);
            }
        }
    }
    chat_response.tools = tools;

    // Add debug information if configured
    if data.config.include_debug_info {
        // Add debug metadata to the response
        if chat_response.id.is_none() {
            chat_response.id = Some(format!("chatcmpl-{}", generate_request_id()));
        }
        if chat_response.object.is_none() {
            chat_response.object = Some("chat.completion".to_string());
        }
        if chat_response.created.is_none() {
            chat_response.created = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
        }
    }

    if data.print_response_converted {
        debug!("\n\n===== Chat Response converted: =====");
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&chat_response).unwrap()
        );
    }

    Ok(web::Json(chat_response))
}
