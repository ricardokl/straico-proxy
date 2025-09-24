use crate::openai_types::OpenAiChatRequest;
use serde_json::Value;
use straico_client::endpoints::chat::ChatResponse;

/// Utilities for processing and enhancing responses from the chat endpoint
pub mod chat_response_utils {
    use super::*;

    /// Enhances a chat response with OpenAI-compatible metadata
    pub fn enhance_chat_response(
        mut response: ChatResponse,
        original_request: &OpenAiChatRequest,
        include_debug: bool,
    ) -> ChatResponse {
        // Ensure response has required OpenAI fields
        if response.id.is_none() {
            response.id = Some(generate_chat_completion_id());
        }

        if response.object.is_none() {
            response.object = Some("chat.completion".to_string());
        }

        if response.created.is_none() {
            response.created = Some(current_timestamp());
        }

        // Set model from request if not present in response
        if response.model.is_empty() && !original_request.model.is_empty() {
            response.model = original_request.model.clone();
        }

        // Add debug information if requested
        if include_debug {
            // Could add additional debug metadata here
        }

        response
    }

    /// Converts a generic JSON response to a ChatResponse with error handling
    pub fn parse_chat_response(json_data: Value) -> Result<ChatResponse, String> {
        serde_json::from_value(json_data)
            .map_err(|e| format!("Failed to parse chat response: {e}"))
    }

    /// Generates a chat completion ID in OpenAI format
    fn generate_chat_completion_id() -> String {
        use rand::distributions::Alphanumeric;
        use rand::Rng;

        let random_part: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(29)
            .map(char::from)
            .collect();

        format!("chatcmpl-{random_part}")
    }

    /// Gets the current Unix timestamp
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Utilities for error handling and response formatting
pub mod error_response_utils {
    use serde::{Deserialize, Serialize};

    /// OpenAI-compatible error response structure
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ErrorResponse {
        pub error: ErrorDetail,
    }

    /// Error detail structure
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ErrorDetail {
        pub message: String,
        #[serde(rename = "type")]
        pub error_type: String,
        pub param: Option<String>,
        pub code: Option<String>,
    }

    /// Creates an OpenAI-compatible error response
    pub fn create_error_response(
        message: String,
        error_type: String,
        param: Option<String>,
        code: Option<String>,
    ) -> ErrorResponse {
        ErrorResponse {
            error: ErrorDetail {
                message,
                error_type,
                param,
                code,
            },
        }
    }

    /// Creates a validation error response
    pub fn create_validation_error(message: String, param: Option<String>) -> ErrorResponse {
        create_error_response(
            message,
            "invalid_request_error".to_string(),
            param,
            Some("validation_failed".to_string()),
        )
    }

    /// Creates a conversion error response
    pub fn create_conversion_error(message: String) -> ErrorResponse {
        create_error_response(
            format!("Content conversion failed: {message}"),
            "invalid_request_error".to_string(),
            None,
            Some("conversion_failed".to_string()),
        )
    }

    /// Creates a server error response
    pub fn create_server_error(message: String) -> ErrorResponse {
        create_error_response(
            format!("Internal server error: {message}"),
            "server_error".to_string(),
            None,
            Some("internal_error".to_string()),
        )
    }
}

/// Utilities for request processing and routing
pub mod request_utils {
    use crate::openai_types::OpenAiChatRequest;

    /// Extracts request metadata for logging and debugging
    pub fn extract_request_metadata(request: &OpenAiChatRequest) -> RequestMetadata {
        RequestMetadata {
            model: request.model.clone(),
            message_count: request.messages.len(),
            has_system_message: request
                .messages
                .first()
                .map(|msg| msg.role == "system")
                .unwrap_or(false),
            has_tools: request.tools.is_some(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: request.stream,
        }
    }

    /// Request metadata for logging and analysis
    #[derive(Debug, Clone)]
    pub struct RequestMetadata {
        pub model: String,
        pub message_count: usize,
        pub has_system_message: bool,
        pub has_tools: bool,
        pub temperature: Option<f32>,
        pub max_tokens: Option<u32>,
        pub stream: bool,
    }
}
