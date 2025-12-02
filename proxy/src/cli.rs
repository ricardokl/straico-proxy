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

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Enable router mode to route requests to different providers based on model prefix
    #[arg(long)]
    pub router: bool,

    /// List available models from Straico API
    #[arg(long)]
    pub list_models: bool,
}
