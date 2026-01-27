use crate::error::ProxyError;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Straico,
}

impl Provider {
    /// Parse provider from model string prefix (before first '/')
    pub fn from_model(model: &str) -> Result<Self, ProxyError> {
        let prefix = model
            .split('/')
            .next()
            .ok_or_else(|| ProxyError::BadRequest("Invalid model format".to_string()))?;

        match prefix.to_lowercase().as_str() {
            "straico" => Ok(Provider::Straico),
            _ => Err(ProxyError::BadRequest(format!(
                "Unknown provider: {}",
                prefix
            ))),
        }
    }

    /// Get the base URL for this provider
    pub fn base_url(&self) -> &'static str {
        match self {
            Provider::Straico => "https://api.straico.com/v2",
        }
    }

    /// Get the environment variable name for the API key
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Provider::Straico => "STRAICO_API_KEY",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Straico => write!(f, "straico"),
        }
    }
}
