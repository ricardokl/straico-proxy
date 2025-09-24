use crate::config::ProxyConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuration file format for persistent settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    /// Proxy server configuration
    pub proxy: ProxyConfig,
    /// Feature flags for experimental functionality
    pub features: FeatureFlags,
    /// Environment-specific settings
    pub environment: EnvironmentConfig,
}

/// Feature flags for controlling experimental and optional functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable the new chat endpoint by default
    pub enable_new_chat_endpoint: bool,
    /// Enable streaming responses (Phase 3 feature)
    pub enable_streaming: bool,
    /// Enable tool calling support (Phase 2 feature)
    pub enable_tool_calls: bool,
    /// Enable request/response caching
    pub enable_caching: bool,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable request logging to file
    pub enable_request_logging: bool,
    /// Enable response compression
    pub enable_compression: bool,
}

/// Environment-specific configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Environment name (development, staging, production)
    pub environment: String,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Server host binding
    pub host: String,
    /// Server port
    pub port: u16,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Maximum concurrent requests
    pub max_concurrent_requests: Option<usize>,
    /// Enable CORS
    pub enable_cors: bool,
    /// Allowed origins for CORS
    pub cors_origins: Vec<String>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_new_chat_endpoint: true,
            enable_streaming: false,  // Phase 3
            enable_tool_calls: false, // Phase 2
            enable_caching: false,
            enable_metrics: false,
            enable_rate_limiting: false,
            enable_request_logging: false,
            enable_compression: false,
        }
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            environment: "development".to_string(),
            log_level: "info".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            request_timeout_seconds: 30,
            max_concurrent_requests: None,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// Configuration manager for loading and saving configuration files
pub struct ConfigManager {
    config_path: String,
    config: ConfigFile,
}

impl ConfigManager {
    /// Creates a new configuration manager
    ///
    /// # Arguments
    /// * `config_path` - Path to the configuration file
    ///
    /// # Returns
    /// A new ConfigManager instance
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        let config_path = config_path.as_ref().to_string_lossy().to_string();
        let config = Self::load_config(&config_path).unwrap_or_default();

        Self {
            config_path,
            config,
        }
    }

    /// Loads configuration from file
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    /// Result containing the loaded configuration or an error
    pub fn load_config(path: &str) -> Result<ConfigFile, Box<dyn std::error::Error + Send + Sync>> {
        if !Path::new(path).exists() {
            return Ok(ConfigFile::default());
        }

        let content = fs::read_to_string(path)?;
        let config: ConfigFile = if path.ends_with(".json") {
            serde_json::from_str(&content)?
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&content)?
        } else {
            // Default to TOML
            toml::from_str(&content)?
        };

        Ok(config)
    }

    /// Saves configuration to file
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn save_config(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = if self.config_path.ends_with(".json") {
            serde_json::to_string_pretty(&self.config)?
        } else if self.config_path.ends_with(".yaml") || self.config_path.ends_with(".yml") {
            serde_yaml::to_string(&self.config)?
        } else {
            // Default to TOML
            toml::to_string_pretty(&self.config)?
        };

        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Gets the current configuration
    pub fn get_config(&self) -> &ConfigFile {
        &self.config
    }

    /// Gets a mutable reference to the configuration
    pub fn get_config_mut(&mut self) -> &mut ConfigFile {
        &mut self.config
    }

    /// Updates the proxy configuration
    pub fn update_proxy_config(&mut self, proxy_config: ProxyConfig) {
        self.config.proxy = proxy_config;
    }

    /// Updates feature flags
    pub fn update_feature_flags(&mut self, feature_flags: FeatureFlags) {
        self.config.features = feature_flags;
    }

    /// Updates environment configuration
    pub fn update_environment_config(&mut self, env_config: EnvironmentConfig) {
        self.config.environment = env_config;
    }

    /// Enables or disables a specific feature flag
    pub fn set_feature_flag(&mut self, flag: &str, enabled: bool) -> Result<(), String> {
        match flag {
            "new_chat_endpoint" => self.config.features.enable_new_chat_endpoint = enabled,
            "streaming" => self.config.features.enable_streaming = enabled,
            "tool_calls" => self.config.features.enable_tool_calls = enabled,
            "caching" => self.config.features.enable_caching = enabled,
            "metrics" => self.config.features.enable_metrics = enabled,
            "rate_limiting" => self.config.features.enable_rate_limiting = enabled,
            "request_logging" => self.config.features.enable_request_logging = enabled,
            "compression" => self.config.features.enable_compression = enabled,
            _ => return Err(format!("Unknown feature flag: {flag}")),
        }
        Ok(())
    }

    /// Gets the value of a specific feature flag
    pub fn get_feature_flag(&self, flag: &str) -> Result<bool, String> {
        match flag {
            "new_chat_endpoint" => Ok(self.config.features.enable_new_chat_endpoint),
            "streaming" => Ok(self.config.features.enable_streaming),
            "tool_calls" => Ok(self.config.features.enable_tool_calls),
            "caching" => Ok(self.config.features.enable_caching),
            "metrics" => Ok(self.config.features.enable_metrics),
            "rate_limiting" => Ok(self.config.features.enable_rate_limiting),
            "request_logging" => Ok(self.config.features.enable_request_logging),
            "compression" => Ok(self.config.features.enable_compression),
            _ => Err(format!("Unknown feature flag: {flag}")),
        }
    }

    /// Validates the current configuration
    pub fn validate_config(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate environment config
        if self.config.environment.port == 0 {
            errors.push("Port cannot be 0".to_string());
        }

        if self.config.environment.request_timeout_seconds == 0 {
            errors.push("Request timeout cannot be 0".to_string());
        }

        if !["development", "staging", "production"]
            .contains(&self.config.environment.environment.as_str())
        {
            errors.push("Environment must be one of: development, staging, production".to_string());
        }

        if !["trace", "debug", "info", "warn", "error"]
            .contains(&self.config.environment.log_level.as_str())
        {
            errors.push("Log level must be one of: trace, debug, info, warn, error".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Creates a default configuration file at the specified path
    pub fn create_default_config<P: AsRef<Path>>(
        path: P,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = ConfigFile::default();
        let path_str = path.as_ref().to_string_lossy();

        let content = if path_str.ends_with(".json") {
            serde_json::to_string_pretty(&config)?
        } else if path_str.ends_with(".yaml") || path_str.ends_with(".yml") {
            serde_yaml::to_string(&config)?
        } else {
            // Default to TOML
            toml::to_string_pretty(&config)?
        };

        fs::write(path, content)?;
        Ok(())
    }

    /// Merges CLI arguments with configuration file settings
    pub fn merge_with_cli_args(&mut self, cli_args: &crate::cli::Cli) {
        // Update proxy config from CLI
        self.config.proxy.include_debug_info = cli_args.include_debug_info;

        // Update environment config from CLI if provided
        self.config.environment.port = cli_args.port;
        self.config.environment.host = cli_args.host.clone();
    }

    /// Gets the effective configuration after merging file and CLI settings
    pub fn get_effective_config(&self) -> EffectiveConfig {
        EffectiveConfig {
            proxy: self.config.proxy.clone(),
            features: self.config.features.clone(),
            environment: self.config.environment.clone(),
        }
    }
}

/// Effective configuration after merging all sources
#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub proxy: ProxyConfig,
    pub features: FeatureFlags,
    pub environment: EnvironmentConfig,
}

impl EffectiveConfig {
    /// Checks if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "new_chat_endpoint" => self.features.enable_new_chat_endpoint,
            "streaming" => self.features.enable_streaming,
            "tool_calls" => self.features.enable_tool_calls,
            "caching" => self.features.enable_caching,
            "metrics" => self.features.enable_metrics,
            "rate_limiting" => self.features.enable_rate_limiting,
            "request_logging" => self.features.enable_request_logging,
            "compression" => self.features.enable_compression,
            _ => false,
        }
    }

    /// Gets all enabled features
    pub fn get_enabled_features(&self) -> Vec<String> {
        let mut enabled = Vec::new();

        if self.features.enable_new_chat_endpoint {
            enabled.push("new_chat_endpoint".to_string());
        }
        if self.features.enable_streaming {
            enabled.push("streaming".to_string());
        }
        if self.features.enable_tool_calls {
            enabled.push("tool_calls".to_string());
        }
        if self.features.enable_caching {
            enabled.push("caching".to_string());
        }
        if self.features.enable_metrics {
            enabled.push("metrics".to_string());
        }
        if self.features.enable_rate_limiting {
            enabled.push("rate_limiting".to_string());
        }
        if self.features.enable_request_logging {
            enabled.push("request_logging".to_string());
        }
        if self.features.enable_compression {
            enabled.push("compression".to_string());
        }

        enabled
    }
}
