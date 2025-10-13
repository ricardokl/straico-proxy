use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Context;
use clap::Parser;
use flexi_logger::{FileSpec, Logger, WriteMode};
use log::{error, info};
use straico_client::client::StraicoClient;
use straico_proxy::{cli::Cli, server};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let mut logger = Logger::try_with_str(&cli.log_level)
        .unwrap_or_else(|e| {
            // Fallback to a default logger if the given level is invalid
            eprintln!(
                "Invalid log level '{}': {}. Defaulting to 'info'.",
                cli.log_level, e
            );
            Logger::try_with_str("info").unwrap()
        })
        .write_mode(WriteMode::BufferAndFlush);

    if cli.log {
        logger = logger
            .log_to_file(FileSpec::default())
            .duplicate_to_stderr(flexi_logger::Duplicate::All);
    } else {
        logger = logger.log_to_stderr();
    }

    logger.start()?;

    // Ensure API key is present
    let api_key = match cli.api_key {
        Some(key) => key,
        None => {
            error!("STRAICO_API_KEY is not set. Please provide it using --api-key or the STRAICO_API_KEY environment variable.");
            return Err(anyhow::anyhow!("STRAICO_API_KEY is not set."));
        }
    };

    let addr = format!("{}:{}", cli.host, cli.port);
    info!("Starting Straico proxy server...");
    info!("Server is running at http://{addr}");
    info!("Completions endpoint is at /v1/chat/completions");

    if cli.debug {
        info!("Debug mode enabled. Raw request and response will be printed to the console.");
    }

    if cli.log {
        info!("Log mode enabled. Raw request and response will be logged to a file.");
    }

    HttpServer::new(move || {
        let app_state = server::AppState {
            client: StraicoClient::new(),
            key: api_key.clone(),
            debug: cli.debug,
            log: cli.log,
        };

        App::new()
            .app_data(web::Data::new(app_state))
            .service(server::openai_chat_completion)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .bind(&addr)
    .with_context(|| format!("Failed to bind to address: {addr}"))?
    .run()
    .await
    .context("Failed to run HTTP server")
}
