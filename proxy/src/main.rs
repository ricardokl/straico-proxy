use std::time::Duration;

use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Context;
use clap::Parser;
use flexi_logger::{Logger, WriteMode};
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
            Logger::try_with_str("info").unwrap_or_else(|e| {
                eprintln!("Failed to initialize fallback logger: {e}");
                std::process::exit(1);
            })
        })
        .write_mode(WriteMode::BufferAndFlush);

    logger = logger.log_to_stderr();

    logger.start()?;

    // Ensure API key is present
    let api_key = match cli.api_key {
        Some(key) => key,
        None => {
            error!("STRAICO_API_KEY is not set. Please provide it using --api-key or the STRAICO_API_KEY environment variable.");
            return Err(anyhow::anyhow!("STRAICO_API_KEY is not set."));
        }
    };

    if cli.list_models {
        let client = StraicoClient::new();
        let response = client
            .models()
            .bearer_auth(&api_key)
            .send()
            .await
            .context("Failed to send request to Straico API")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch models: {} - {}", status, text);
        }

        let models_response: straico_client::endpoints::models::ModelsResponse = response
            .json()
            .await
            .context("Failed to parse models response")?;

        let json_output = serde_json::to_string_pretty(&models_response)
            .context("Failed to serialize models response")?;
        println!("{}", json_output);
        return Ok(());
    }

    let addr = format!("{}:{}", cli.host, cli.port);
    info!("Starting Straico proxy server...");
    info!("Server is running at http://{addr}");



    info!("Completions endpoint is at /v1/chat/completions");

    let client = StraicoClient::builder()
        .pool_max_idle_per_host(25)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(90))
        .timeout(Duration::from_secs(90))
        .build()?;

    HttpServer::new(move || {
        let app_state = server::AppState {
            client: client.clone(),
            key: api_key.clone(),
            heartbeat_char: cli.heartbeat_char,
        };

        let app = App::new().app_data(web::Data::new(app_state));
        app.service(server::openai_chat_completion)
            .service(server::model_handler)
            .default_service(web::to(HttpResponse::NotFound))
            .service(server::models_handler)
    })
    .bind(&addr)
    .with_context(|| format!("Failed to bind to address: {addr}"))?
    .run()
    .await
    .context("Failed to run HTTP server")
}
