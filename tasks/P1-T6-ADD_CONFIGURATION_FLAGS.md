# P1-T6: Add Configuration and Feature Flags

## Objective
Implement comprehensive configuration options and feature flags to control endpoint selection, enable testing, and provide smooth migration paths.

## Background
Users need flexible options to choose between endpoints, test new functionality, and gradually migrate from old to new endpoint.

## Tasks

### 1. Extend CLI Configuration
**File**: `proxy/src/main.rs`

**Enhanced CLI Options**:
```rust
#[derive(Parser)]
#[command(
    name = "straico-proxy",
    about = "A proxy server for Straico API that provides OpenAI-compatible endpoints",
    version
)]
struct Cli {
    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, default_value = "8000")]
    port: u16,

    /// API key for Straico (alternatively use STRAICO_API_KEY env var)
    #[arg(long, env = "STRAICO_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    /// Log to file (proxy.log)
    #[arg(long)]
    log_to_file: bool,

    /// Log to standard output
    #[arg(long)]
    log_to_stdout: bool,

    /// Print the raw request
    #[arg(long)]
    print_request_raw: bool,
    
    /// Print the request after converting to Straico format
    #[arg(long)]
    print_request_converted: bool,
    
    /// Print the raw response from Straico
    #[arg(long)]
    print_response_raw: bool,
    
    /// Print the response after converting to OpenAI format
    #[arg(long)]
    print_response_converted: bool,

    // === NEW ENDPOINT CONFIGURATION ===
    
    /// Use new chat endpoint by default
    #[arg(long, env = "STRAICO_USE_CHAT_ENDPOINT")]
    use_chat_endpoint: bool,
    
    /// Force old completion endpoint (overrides use_chat_endpoint)
    #[arg(long, env = "STRAICO_FORCE_COMPLETION_ENDPOINT")]
    force_completion_endpoint: bool,
    
    /// Enable both endpoints simultaneously for testing
    #[arg(long)]
    enable_dual_endpoints: bool,
    
    /// Chat endpoint URL override
    #[arg(long, env = "STRAICO_CHAT_ENDPOINT_URL")]
    chat_endpoint_url: Option<String>,
    
    /// Completion endpoint URL override  
    #[arg(long, env = "STRAICO_COMPLETION_ENDPOINT_URL")]
    completion_endpoint_url: Option<String>,
    
    /// Enable experimental features
    #[arg(long)]
    enable_experimental: bool,
}
```

### 2. Create Configuration Module
**File**: `proxy/src/config.rs` (new file)

**Configuration Management**:
```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub endpoints: EndpointConfig,
    pub logging: LoggingConfig,
    pub features: FeatureConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EndpointConfig {
    pub use_chat_endpoint: bool,
    pub force_completion_endpoint: bool,
    pub enable_dual_endpoints: bool,
    pub chat_endpoint_url: String,
    pub completion_endpoint_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub print_request_raw: bool,
    pub print_request_converted: bool,
    pub print_response_raw: bool,
    pub print_response_converted: bool,
    pub log_to_file: bool,
    pub log_to_stdout: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub enable_experimental: bool,
    pub enable_tool_calls: bool,
    pub enable_streaming: bool,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            use_chat_endpoint: false,
            force_completion_endpoint: false,
            enable_dual_endpoints: false,
            chat_endpoint_url: "https://api.straico.com/v0/chat/completions".to_string(),
            completion_endpoint_url: "https://api.straico.com/v1/prompt/completion".to_string(),
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            enable_experimental: false,
            enable_tool_calls: false,
            enable_streaming: false,
        }
    }
}

impl From<crate::Cli> for ProxyConfig {
    fn from(cli: crate::Cli) -> Self {
        Self {
            endpoints: EndpointConfig {
                use_chat_endpoint: cli.use_chat_endpoint,
                force_completion_endpoint: cli.force_completion_endpoint,
                enable_dual_endpoints: cli.enable_dual_endpoints,
                chat_endpoint_url: cli.chat_endpoint_url
                    .unwrap_or_else(|| EndpointConfig::default().chat_endpoint_url),
                completion_endpoint_url: cli.completion_endpoint_url
                    .unwrap_or_else(|| EndpointConfig::default().completion_endpoint_url),
            },
            logging: LoggingConfig {
                print_request_raw: cli.print_request_raw,
                print_request_converted: cli.print_request_converted,
                print_response_raw: cli.print_response_raw,
                print_response_converted: cli.print_response_converted,
                log_to_file: cli.log_to_file,
                log_to_stdout: cli.log_to_stdout,
            },
            features: FeatureConfig {
                enable_experimental: cli.enable_experimental,
                enable_tool_calls: false, // Will be enabled in Phase 2
                enable_streaming: false,  // Will be enabled in Phase 3
            },
        }
    }
}
```

### 3. Update AppState
**File**: `proxy/src/main.rs`

**Enhanced AppState**:
```rust
#[derive(Clone)]
struct AppState {
    client: StraicoClient,
    key: String,
    config: ProxyConfig,
}

impl AppState {
    fn should_use_chat_endpoint(&self) -> bool {
        if self.config.endpoints.force_completion_endpoint {
            false
        } else {
            self.config.endpoints.use_chat_endpoint
        }
    }
    
    fn chat_endpoint_url(&self) -> &str {
        &self.config.endpoints.chat_endpoint_url
    }
    
    fn completion_endpoint_url(&self) -> &str {
        &self.config.endpoints.completion_endpoint_url
    }
}
```

### 4. Add Runtime Configuration
**File**: `proxy/src/config.rs`

**Configuration File Support**:
```rust
use std::path::Path;
use std::fs;

impl ProxyConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: ProxyConfig = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Merge CLI options with file configuration
    pub fn merge_with_cli(mut self, cli: crate::Cli) -> Self {
        // CLI options override file configuration
        if cli.use_chat_endpoint {
            self.endpoints.use_chat_endpoint = true;
        }
        if cli.force_completion_endpoint {
            self.endpoints.force_completion_endpoint = true;
        }
        // ... merge other options
        self
    }
}
```

### 5. Add Environment Variable Support
**File**: `proxy/src/main.rs`

**Environment Configuration**:
```rust
fn load_configuration(cli: Cli) -> ProxyConfig {
    // Try to load from config file first
    let mut config = if let Ok(config_path) = std::env::var("STRAICO_CONFIG_FILE") {
        ProxyConfig::from_file(config_path)
            .unwrap_or_else(|_| ProxyConfig::default())
    } else if Path::new("straico-proxy.toml").exists() {
        ProxyConfig::from_file("straico-proxy.toml")
            .unwrap_or_else(|_| ProxyConfig::default())
    } else {
        ProxyConfig::default()
    };
    
    // Merge with CLI options (CLI takes precedence)
    config = config.merge_with_cli(cli);
    
    config
}
```

### 6. Add Configuration Validation
**File**: `proxy/src/config.rs`

**Validation Functions**:
```rust
impl ProxyConfig {
    pub fn validate(&self) -> Result<(), String> {
        // Validate endpoint URLs
        if !self.endpoints.chat_endpoint_url.starts_with("http") {
            return Err("Chat endpoint URL must start with http or https".to_string());
        }
        
        if !self.endpoints.completion_endpoint_url.starts_with("http") {
            return Err("Completion endpoint URL must start with http or https".to_string());
        }
        
        // Validate conflicting options
        if self.endpoints.force_completion_endpoint && self.endpoints.use_chat_endpoint {
            log::warn!("force_completion_endpoint overrides use_chat_endpoint");
        }
        
        Ok(())
    }
}
```

### 7. Add Configuration Display
**File**: `proxy/src/main.rs`

**Startup Information**:
```rust
fn print_startup_info(config: &ProxyConfig, addr: &str) {
    info!("Starting Straico proxy server...");
    info!("Server is running at http://{}", addr);
    info!("Completions endpoint is at /v1/chat/completions");
    
    if config.endpoints.force_completion_endpoint {
        info!("Using COMPLETION endpoint (forced)");
    } else if config.endpoints.use_chat_endpoint {
        info!("Using CHAT endpoint");
    } else {
        info!("Using COMPLETION endpoint (default)");
    }
    
    if config.endpoints.enable_dual_endpoints {
        info!("Dual endpoint mode enabled");
    }
    
    if config.features.enable_experimental {
        info!("Experimental features enabled");
    }
    
    // Print debug settings
    if config.logging.print_request_raw {
        info!("Printing raw requests");
    }
    if config.logging.print_request_converted {
        info!("Printing converted requests");
    }
    if config.logging.print_response_raw {
        info!("Printing raw responses");
    }
    if config.logging.print_response_converted {
        info!("Printing converted responses");
    }
}
```

## Deliverables

1. **Enhanced CLI**:
   - New endpoint selection flags
   - URL override options
   - Feature toggle flags
   - Environment variable support

2. **Configuration Module**:
   - Structured configuration types
   - File-based configuration
   - Configuration validation
   - CLI/file/env merging

3. **Runtime Management**:
   - Dynamic endpoint selection
   - Feature flag checking
   - Configuration validation
   - Startup information display

4. **Documentation**:
   - Configuration file examples
   - Environment variable list
   - Usage examples

## Success Criteria

- [ ] CLI accepts all new configuration options
- [ ] Configuration can be loaded from files
- [ ] Environment variables work correctly
- [ ] Configuration validation prevents invalid setups
- [ ] Endpoint selection works as expected
- [ ] Feature flags control functionality
- [ ] Startup info shows current configuration
- [ ] Documentation is comprehensive

## Time Estimate
**Duration**: 2-3 hours

## Dependencies
- **P1-T5**: Update Proxy Server for New Endpoint

## Next Task
**P1-T7**: Testing and Validation

## Notes
- Provide sensible defaults for all options
- Ensure CLI options override file configuration
- Add comprehensive validation
- Consider adding configuration hot-reload in future
- Document all configuration options clearly