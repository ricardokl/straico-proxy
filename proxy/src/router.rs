use crate::error::CustomError;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Straico,
    SambaNova,
    Cerebras,
    Groq,
}

impl Provider {
    /// Parse provider from model string prefix (before first '/')
    pub fn from_model(model: &str) -> Result<Self, CustomError> {
        let prefix = model
            .split('/')
            .next()
            .ok_or_else(|| CustomError::BadRequest("Invalid model format".to_string()))?;

        match prefix.to_lowercase().as_str() {
            "straico" => Ok(Provider::Straico),
            "sambanova" => Ok(Provider::SambaNova),
            "cerebras" => Ok(Provider::Cerebras),
            "groq" => Ok(Provider::Groq),
            _ => Err(CustomError::BadRequest(format!(
                "Unknown provider: {}",
                prefix
            ))),
        }
    }

    /// Get the base URL for this provider
    pub fn base_url(&self) -> &'static str {
        match self {
            Provider::Straico => "https://api.straico.com/v1",
            Provider::SambaNova => "https://api.sambanova.ai/v1/chat/completions",
            Provider::Cerebras => "https://api.cerebras.ai/v1/chat/completions",
            Provider::Groq => "https://api.groq.com/openai/v1/chat/completions",
        }
    }

    /// Get the environment variable name for the API key
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Provider::Straico => "STRAICO_API_KEY",
            Provider::SambaNova => "SAMBANOVA_API_KEY",
            Provider::Cerebras => "CEREBRAS_API_KEY",
            Provider::Groq => "GROQ_API_KEY",
        }
    }

    /// Check if this provider requires response conversion
    pub fn needs_conversion(&self) -> bool {
        matches!(self, Provider::Straico)
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Straico => write!(f, "straico"),
            Provider::SambaNova => write!(f, "sambanova"),
            Provider::Cerebras => write!(f, "cerebras"),
            Provider::Groq => write!(f, "groq"),
        }
    }
}
