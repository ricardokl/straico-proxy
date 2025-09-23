use serde::{Deserialize, Serialize};

/// Configuration options for the proxy server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether to enable streaming for chat responses
    pub enable_chat_streaming: bool,
    /// Whether to include debug information in responses
    pub include_debug_info: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enable_chat_streaming: false, // Will be implemented in Phase 3
            include_debug_info: false,
        }
    }
}

impl ProxyConfig {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to enable streaming
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.enable_chat_streaming = enabled;
        self
    }

    /// Sets whether to include debug information
    pub fn with_debug_info(mut self, enabled: bool) -> Self {
        self.include_debug_info = enabled;
        self
    }
}
