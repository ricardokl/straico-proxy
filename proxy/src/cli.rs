use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "straico-proxy",
    about = "A proxy server for Straico API that provides OpenAI-compatible endpoints",
    version
)]
pub struct Cli {
    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on
    #[arg(long, default_value = "8000")]
    pub port: u16,

    /// API key for Straico (alternatively use STRAICO_API_KEY env var)
    #[arg(long, env = "STRAICO_API_KEY", hide_env_values = true)]
    pub api_key: Option<String>,

    /// Configuration file path
    #[arg(long, default_value = "config.toml")]
    pub config: String,

    /// Create a default configuration file and exit
    #[arg(long)]
    pub create_config: bool,

    /// Log to file (proxy.log)
    #[arg(long)]
    pub log_to_file: bool,

    /// Log to standard output
    #[arg(long)]
    pub log_to_stdout: bool,

    /// Print the raw request
    #[arg(long)]
    pub print_request_raw: bool,
    /// Print the request after converting to Straico format
    #[arg(long)]
    pub print_request_converted: bool,
    /// Print the raw response from Straico
    #[arg(long)]
    pub print_response_raw: bool,
    /// Print the response after converting to OpenAI format
    #[arg(long)]
    pub print_response_converted: bool,

    /// Include debug information in responses
    #[arg(long)]
    pub include_debug_info: bool,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Environment (development, staging, production)
    #[arg(long)]
    pub environment: Option<String>,

    /// Enable feature flag
    #[arg(long, action = clap::ArgAction::Append)]
    pub enable_feature: Vec<String>,

    /// Disable feature flag
    #[arg(long, action = clap::ArgAction::Append)]
    pub disable_feature: Vec<String>,

    /// Request timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,
}
