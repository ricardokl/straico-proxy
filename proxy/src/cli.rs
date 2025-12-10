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

    // The parser adds "[env STRAICO_API_KEY]" in the end
    /// Set API key for Straico or use
    #[arg(long, env = "STRAICO_API_KEY", hide_env_values = true)]
    pub api_key: Option<String>,

    /// List available models from Straico API
    #[arg(long)]
    pub list_models: bool,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Enable router mode
    #[arg(long)]
    pub router: bool,
}
