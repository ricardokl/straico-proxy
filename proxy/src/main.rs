use actix_web::{App, HttpResponse, HttpServer, web};
use anyhow::Context;
use clap::Parser;
use flexi_logger::{FileSpec, Logger, WriteMode};
use log::{info, warn};
use straico_client::client::StraicoClient;
use config::ProxyConfig;
mod config;
mod content_conversion;
mod error;
mod openai_types;
mod response_utils;
mod server;
mod streaming;

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

    /// Use the new chat endpoint by default
    #[arg(long)]
    use_new_chat_endpoint: bool,

    /// Enable request validation
    #[arg(long)]
    validate_requests: bool,

    /// Include debug information in responses
    #[arg(long)]
    include_debug_info: bool,
}

/// Represents the application state shared across HTTP request handlers.
///
/// This struct contains all the necessary components for handling requests,
/// including the Straico API client, authentication key, and debug settings.
#[derive(Clone)]
struct AppState {
    /// The Straico API client used for making requests
    client: StraicoClient,
    /// API authentication key for Straico
    key: String,
    /// Proxy configuration settings
    config: ProxyConfig,
    print_request_raw: bool,
    print_request_converted: bool,
    print_response_raw: bool,
    print_response_converted: bool,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let log_level = if cli.print_request_raw
        || cli.print_request_converted
        || cli.print_response_raw
        || cli.print_response_converted
    {
        "debug"
    } else {
        "info"
    };

    let mut logger = Logger::try_with_str(log_level)?.write_mode(WriteMode::BufferAndFlush);

    if cli.log_to_file {
        logger = logger.log_to_file(FileSpec::default());
        if cli.log_to_stdout {
            logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::All);
        }
    } else if cli.log_to_stdout {
        logger = logger.log_to_stderr();
    } else {
        // Default behavior: log info to stdout to show server is running
        logger = logger.log_to_stderr();
    }

    logger.start()?;

    let api_key = match cli.api_key {
        Some(key) => key,
        None => {
            warn!("API key not set, exiting");
            return Ok(());
        }
    };

    let addr = format!("{}:{}", cli.host, cli.port);
    info!("Starting Straico proxy server...");
    info!("Server is running at http://{}", addr);
    info!("Completions endpoint is at /v1/chat/completions");
    if cli.print_request_raw {
        info!("Printing raw requests");
    }
    if cli.print_request_converted {
        info!("Printing converted requests");
    }
    if cli.print_response_raw {
        info!("Printing raw responses");
    }
    if cli.print_response_converted {
        info!("Printing converted responses");
    }

    let proxy_config = ProxyConfig::new()
        .with_new_chat_endpoint(cli.use_new_chat_endpoint)
        .with_validation(cli.validate_requests)
        .with_debug_info(cli.include_debug_info);

    info!("Configuration:");
    info!("  - Use new chat endpoint: {}", proxy_config.use_new_chat_endpoint);
    info!("  - Validate requests: {}", proxy_config.validate_requests);
    info!("  - Include debug info: {}", proxy_config.include_debug_info);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                client: StraicoClient::new(),
                key: api_key.clone(),
                config: proxy_config.clone(),
                print_request_raw: cli.print_request_raw,
                print_request_converted: cli.print_request_converted,
                print_response_raw: cli.print_response_raw,
                print_response_converted: cli.print_response_converted,
            }))
            .service(server::openai_completion)
            .service(server::openai_chat_completion)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .bind(addr)?
    .run()
    .await
    .context("Failed to run HTTP server")
}
