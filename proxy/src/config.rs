use serde::{Deserialize, Serialize};

/// Configuration options for the proxy server
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether to enable streaming for chat responses
    pub enable_chat_streaming: bool,
    /// Whether to include debug information in responses
    pub include_debug_info: bool,
}

impl ProxyConfig {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
}