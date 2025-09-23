use crate::openai_types::OpenAiChatRequest;
use crate::AppState;

pub fn should_use_new_endpoint(
    request: &OpenAiChatRequest,
    config: &AppState,
) -> bool {
    // Always use new endpoint if configured
    if config.use_new_chat_endpoint {
        return true;
    }

    // Use new endpoint for tool calls if forced
    if config.force_new_endpoint_for_tools && request.tools.is_some() {
        return true;
    }

    // Default to legacy endpoint
    false
}

use crate::config::ProxyConfig;

pub fn validate_request_for_endpoint(
    request: &OpenAiChatRequest,
    _use_new_endpoint: bool,
    config: &ProxyConfig,
) -> Result<(), String> {
    config.validate_chat_request(request)
}
