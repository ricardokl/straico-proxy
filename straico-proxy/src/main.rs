use actix_web::{App, HttpResponse, HttpServer, web};
use anyhow::Context;
use clap::Parser;
use flexi_logger::{FileSpec, Logger, WriteMode};
use log::{info, warn};
use straico_client::client::StraicoClient;
mod error;
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

    let logger = Logger::try_with_str(log_level)?
        .log_to_file(FileSpec::default())
        .write_mode(WriteMode::BufferAndFlush)
        .duplicate_to_stderr(if cli.print_request_raw
            || cli.print_request_converted
            || cli.print_response_raw
            || cli.print_response_converted
        {
            flexi_logger::Duplicate::All
        } else {
            flexi_logger::Duplicate::Info
        });

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

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                client: StraicoClient::new(),
                key: api_key.clone(),
                print_request_raw: cli.print_request_raw,
                print_request_converted: cli.print_request_converted,
                print_response_raw: cli.print_response_raw,
                print_response_converted: cli.print_response_converted,
            }))
            .service(server::openai_completion)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .bind(addr)?
    .run()
    .await
    .context("Failed to run HTTP server")
}
