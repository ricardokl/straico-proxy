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
        serde_json::from_value(json_data).map_err(|e| format!("Failed to parse chat response: {e}"))
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
