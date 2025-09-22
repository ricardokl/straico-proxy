use crate::openai_types::OpenAiChatRequest;
use straico_client::endpoints::chat::ChatResponse;
use serde_json::Value;

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

    /// Validates that a chat response has all required OpenAI fields
    pub fn validate_chat_response(response: &ChatResponse) -> Result<(), String> {
        if response.choices.is_empty() {
            return Err("Response must contain at least one choice".to_string());
        }

        for (i, choice) in response.choices.iter().enumerate() {
            if choice.message.role.is_empty() {
                return Err(format!("Choice {} message must have a role", i));
            }

            if choice.finish_reason.is_empty() {
                return Err(format!("Choice {} must have a finish reason", i));
            }

            // Validate that choice has either content or tool calls
            if choice.message.content.is_none() && choice.message.tool_calls.is_none() {
                return Err(format!("Choice {} message must have either content or tool calls", i));
            }
        }

        Ok(())
    }

    /// Converts a generic JSON response to a ChatResponse with error handling
    pub fn parse_chat_response(json_data: Value) -> Result<ChatResponse, String> {
        serde_json::from_value(json_data)
            .map_err(|e| format!("Failed to parse chat response: {}", e))
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
        
        format!("chatcmpl-{}", random_part)
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
    use super::*;
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
            format!("Content conversion failed: {}", message),
            "invalid_request_error".to_string(),
            None,
            Some("conversion_failed".to_string()),
        )
    }

    /// Creates a server error response
    pub fn create_server_error(message: String) -> ErrorResponse {
        create_error_response(
            format!("Internal server error: {}", message),
            "server_error".to_string(),
            None,
            Some("internal_error".to_string()),
        )
    }
}

/// Utilities for request processing and routing
pub mod request_utils {
    use crate::config::{EndpointRoute, ProxyConfig};
    use crate::openai_types::OpenAiChatRequest;

    /// Determines the best endpoint route for a given request
    pub fn determine_endpoint_route(
        request: &OpenAiChatRequest,
        config: &ProxyConfig,
    ) -> EndpointRoute {
        config.determine_endpoint_route(request)
    }

    /// Validates and preprocesses a chat request
    pub fn preprocess_chat_request(
        request: &mut OpenAiChatRequest,
        config: &ProxyConfig,
    ) -> Result<(), String> {
        // Validate against configuration limits
        config.validate_chat_request(request)?;

        // Normalize request fields
        if request.model.trim().is_empty() {
            return Err("Model field cannot be empty".to_string());
        }

        // Ensure messages are not empty
        if request.messages.is_empty() {
            return Err("Messages array cannot be empty".to_string());
        }

        // Validate temperature range
        if let Some(temp) = request.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err("Temperature must be between 0.0 and 2.0".to_string());
            }
        }

        // Validate max_tokens
        if let Some(tokens) = request.max_tokens {
            if tokens == 0 {
                return Err("max_tokens must be greater than 0".to_string());
            }
        }

        Ok(())
    }

    /// Extracts request metadata for logging and debugging
    pub fn extract_request_metadata(request: &OpenAiChatRequest) -> RequestMetadata {
        RequestMetadata {
            model: request.model.clone(),
            message_count: request.messages.len(),
            has_system_message: request.messages.first()
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