use serde::{Deserialize, Serialize};

/// Configuration options for the proxy server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether to use the new chat endpoint by default
    pub use_new_chat_endpoint: bool,
    /// Force tool calls to use new endpoint (even if legacy is default)
    pub force_new_endpoint_for_tools: bool,
    /// Whether to enable streaming for chat responses
    pub enable_chat_streaming: bool,
    /// Whether to validate OpenAI requests before conversion
    pub validate_requests: bool,
    /// Whether to include debug information in responses
    pub include_debug_info: bool,
    /// Maximum number of messages allowed in a chat request
    pub max_messages_per_request: Option<usize>,
    /// Maximum content length per message
    pub max_content_length: Option<usize>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            use_new_chat_endpoint: true,
            force_new_endpoint_for_tools: false,
            enable_chat_streaming: false, // Will be implemented in Phase 3
            validate_requests: true,
            include_debug_info: false,
            max_messages_per_request: Some(100),
            max_content_length: Some(10000),
        }
    }
}

/// Endpoint routing configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum EndpointRoute {
    /// Use the legacy completion endpoint
    Legacy,
    /// Use the new chat endpoint
    #[default]
    NewChat,
    /// Auto-select based on request characteristics
    Auto,
}


impl ProxyConfig {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to use the new chat endpoint
    pub fn with_new_chat_endpoint(mut self, enabled: bool) -> Self {
        self.use_new_chat_endpoint = enabled;
        self
    }

    /// Sets whether to enable streaming
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.enable_chat_streaming = enabled;
        self
    }

    /// Sets whether to validate requests
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validate_requests = enabled;
        self
    }

    /// Sets whether to include debug information
    pub fn with_debug_info(mut self, enabled: bool) -> Self {
        self.include_debug_info = enabled;
        self
    }

    /// Sets the maximum number of messages per request
    pub fn with_max_messages(mut self, max: Option<usize>) -> Self {
        self.max_messages_per_request = max;
        self
    }

    /// Sets the maximum content length per message
    pub fn with_max_content_length(mut self, max: Option<usize>) -> Self {
        self.max_content_length = max;
        self
    }

    /// Validates a chat request against the configuration limits
    pub fn validate_chat_request(&self, request: &crate::openai_types::OpenAiChatRequest) -> Result<(), String> {
        log::info!("Validating chat request: {:?}", request);
        if !self.validate_requests {
            log::info!("Request validation is disabled.");
            return Ok(());
        }

        if request.model.is_empty() {
            let err_msg = "Model field cannot be empty".to_string();
            log::warn!("{}", err_msg);
            return Err(err_msg);
        }

        // Check message count limit
        if request.messages.is_empty() {
            let err_msg = "Messages array cannot be empty".to_string();
            log::warn!("{}", err_msg);
            return Err(err_msg);
        }
        if let Some(max_messages) = self.max_messages_per_request {
            if request.messages.len() > max_messages {
                let err_msg = format!(
                    "Too many messages: {} (max: {})",
                    request.messages.len(),
                    max_messages
                );
                log::warn!("{}", err_msg);
                return Err(err_msg);
            }
        }

        // Check content length limits
        if let Some(max_length) = self.max_content_length {
            for (i, message) in request.messages.iter().enumerate() {
                let content_length = message.content.to_string().len();
                if content_length > max_length {
                    let err_msg = format!(
                        "Message {} content too long: {} characters (max: {})",
                        i,
                        content_length,
                        max_length
                    );
                    log::warn!("{}", err_msg);
                    return Err(err_msg);
                }
            }
        }
        if let Some(temperature) = request.temperature {
            if !(0.0..=2.0).contains(&temperature) {
                let err_msg = format!("Invalid temperature: {}", temperature);
                log::warn!("{}", err_msg);
                return Err(err_msg);
            }
        }

        log::info!("Request validation successful.");
        Ok(())
    }

    /// Determines which endpoint to route the request to
    pub fn determine_endpoint_route(&self, _request: &crate::openai_types::OpenAiChatRequest) -> EndpointRoute {
        if self.use_new_chat_endpoint {
            EndpointRoute::NewChat
        } else {
            EndpointRoute::Legacy
        }
    }
}