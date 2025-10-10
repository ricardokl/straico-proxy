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
    #[arg(long, default_value = "info")]
    pub log_level: String,
}
