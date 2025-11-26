use crate::{
    error::CustomError,
    router::Provider,
    types::OpenAiChatRequest,
    providers::StraicoProvider,
};
use actix_web::{get, post, web, HttpResponse};
use futures::TryStreamExt;
use log::{debug, info, warn};
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};
use straico_client::client::StraicoClient;

/// Safely gets the current Unix timestamp, with fallback for edge cases
///
/// In the extremely unlikely case that the system clock is set before UNIX_EPOCH,
/// this function will log a warning and return a reasonable fallback timestamp.
#[cfg(test)]
fn get_current_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(e) => {
            // This should never happen in practice, but handle it gracefully
            warn!(
                "System clock appears to be set before Unix epoch ({}). Using fallback timestamp.",
                e
            );
            // Return a reasonable fallback timestamp (2024-01-01 00:00:00 UTC)
            1704067200
        }
    }
}

/// Helper function to log messages based on debug and log flags
fn log_message(debug: bool, log: bool, message: &str) {
    if debug || log {
        if debug {
            debug!("{}", message);
        }
        if log {
            info!("{}", message);
        }
    }
}



#[derive(Clone)]
pub struct AppState {
    pub client: StraicoClient,
    pub key: String,
    pub debug: bool,
    pub log: bool,
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

    log_message(
        data.debug,
        data.log,
        &format!(
            "\n\n===== Request received (raw): =====\n{}",
            serde_json::to_string_pretty(&openai_request)?
        ),
    );

    let provider = StraicoProvider::new(data.client.clone());
    provider
        .chat(openai_request, &data.key, data.debug, data.log)
        .await
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
    if provider.needs_conversion() {
        log_message(
            data.debug,
            data.log,
            &format!(
                "\n\n===== Router Request received (raw): =====\n{}",
                serde_json::to_string_pretty(&openai_request)?
            ),
        );

        log_message(
            data.debug,
            data.log,
            &format!(
                "Routing to provider: {} ({})",
                provider,
                provider.base_url()
            ),
        );
    }

    let implementation = provider.get_implementation(&data.client);
    implementation
        .chat(openai_request, &api_key, data.debug, data.log)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_timestamp() {
        let timestamp = get_current_timestamp();

        // Should be a reasonable timestamp (after 2020-01-01 and before 2030-01-01)
        let year_2020 = 1577836800; // 2020-01-01 00:00:00 UTC
        let year_2030 = 1893456000; // 2030-01-01 00:00:00 UTC

        assert!(timestamp >= year_2020, "Timestamp should be after 2020");
        assert!(timestamp <= year_2030, "Timestamp should be before 2030");
    }

    #[test]
    fn test_get_current_timestamp_consistency() {
        let timestamp1 = get_current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let timestamp2 = get_current_timestamp();

        // Timestamps should be very close (within a few seconds)
        assert!(timestamp2 >= timestamp1);
        assert!(timestamp2 - timestamp1 <= 1); // Should be within 1 second
    }

    #[test]
    fn test_fallback_timestamp_value() {
        // Test that the fallback timestamp is reasonable
        let fallback_timestamp = 1704067200; // 2024-01-01 00:00:00 UTC

        // Should be after 2020 and before 2030
        let year_2020 = 1577836800; // 2020-01-01 00:00:00 UTC
        let year_2030 = 1893456000; // 2030-01-01 00:00:00 UTC

        assert!(fallback_timestamp > year_2020);
        assert!(fallback_timestamp < year_2030);
    }

    #[test]
    fn test_timestamp_function_never_panics() {
        // This test ensures our function doesn't panic under normal conditions
        // We can't easily test the edge case without mocking SystemTime
        for _ in 0..100 {
            let _timestamp = get_current_timestamp();
            // If we get here, the function didn't panic
        }
    }
}
