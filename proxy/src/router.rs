use crate::error::ProxyError;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GenericProviderType {
    SambaNova,
    Cerebras,
    Groq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Straico,
    Generic(GenericProviderType),
}

impl GenericProviderType {
    pub fn base_url(&self) -> &'static str {
        match self {
            GenericProviderType::SambaNova => "https://api.sambanova.ai/v1/chat/completions",
            GenericProviderType::Cerebras => "https://api.cerebras.ai/v1/chat/completions",
            GenericProviderType::Groq => "https://api.groq.com/openai/v1/chat/completions",
        }
    }
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
            "sambanova" => Ok(Provider::Generic(GenericProviderType::SambaNova)),
            "cerebras" => Ok(Provider::Generic(GenericProviderType::Cerebras)),
            "groq" => Ok(Provider::Generic(GenericProviderType::Groq)),
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
            Provider::Generic(p) => p.base_url(),
        }
    }

    /// Get the environment variable name for the API key
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Provider::Straico => "STRAICO_API_KEY",
            Provider::Generic(p) => match p {
                GenericProviderType::SambaNova => "SAMBANOVA_API_KEY",
                GenericProviderType::Cerebras => "CEREBRAS_API_KEY",
                GenericProviderType::Groq => "GROQ_API_KEY",
            },
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Straico => write!(f, "straico"),
            Provider::Generic(p) => match p {
                GenericProviderType::SambaNova => write!(f, "sambanova"),
                GenericProviderType::Cerebras => write!(f, "cerebras"),
                GenericProviderType::Groq => write!(f, "groq"),
            },
        }
    }
}
