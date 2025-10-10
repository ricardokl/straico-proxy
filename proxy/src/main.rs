use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Context;
use clap::Parser;
use flexi_logger::{FileSpec, Logger, WriteMode};
use log::{error, info};
use straico_client::client::StraicoClient;
use straico_proxy::{cli::Cli, config::ProxyConfig, server};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let mut logger = Logger::try_with_str(&cli.log_level)
        .unwrap_or_else(|e| {
            // Fallback to a default logger if the given level is invalid
            eprintln!("Invalid log level '{}': {}. Defaulting to 'info'.", cli.log_level, e);
            Logger::try_with_str("info").unwrap()
        })
        .write_mode(WriteMode::BufferAndFlush);

    if cli.log_to_file {
        logger = logger.log_to_file(FileSpec::default());
        if cli.log_to_stdout {
            logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::All);
        }
    } else {
        // Default behavior: log to stdout if no file logging is specified,
        // or if both file and stdout are requested.
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
    if cli.include_debug_info {
        info!("Including debug info in responses");
    }

    // Create ProxyConfig from CLI arguments
    let proxy_config = ProxyConfig {
        enable_chat_streaming: false, // Streaming is not implemented
        include_debug_info: cli.include_debug_info,
    };

    HttpServer::new(move || {
        let app_state = server::AppState {
            client: StraicoClient::new(),
            key: api_key.clone(),
            config: proxy_config.clone(),
            print_request_raw: cli.print_request_raw,
            print_request_converted: cli.print_request_converted,
            print_response_raw: cli.print_response_raw,
            print_response_converted: cli.print_response_converted,
        };

        App::new()
            .app_data(web::Data::new(app_state))
            .service(server::openai_chat_completion)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .bind(&addr)
    .with_context(|| format!("Failed to bind to address: {}", addr))?
    .run()
    .await
    .context("Failed to run HTTP server")
}