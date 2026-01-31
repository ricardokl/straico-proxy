use std::time::Duration;

use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use anyhow::Context;
use clap::Parser;
use flexi_logger::{Logger, WriteMode};
use log::{error, info};
use straico_client::client::StraicoClient;
use straico_proxy::{cli::Cli, server};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set up logging with actix-http at error level to catch parse errors
    let log_spec = format!("{},actix_http::h1::dispatcher=error", &cli.log_level);
    let mut logger = Logger::try_with_str(&log_spec)
        .unwrap_or_else(|e| {
            // Fallback to a default logger if the given level is invalid
            eprintln!(
                "Invalid log level '{}': {}. Defaulting to 'info'.",
                cli.log_level, e
            );
            Logger::try_with_str("info,actix_http::h1::dispatcher=error").unwrap_or_else(|e| {
                eprintln!("Failed to initialize fallback logger: {e}");
                std::process::exit(1);
            })
        })
        .write_mode(WriteMode::BufferAndFlush)
        .format(|w, now, record| {
            // Intercept actix-http parse errors and add helpful context
            if record.target() == "actix_http::h1::dispatcher"
                && record.level() == log::Level::Error
            {
                let msg = format!("{}", record.args());
                if msg.contains("invalid Header") {
                    // Log the original error first
                    write!(
                        w,
                        "[{}] ERROR [{}] {}",
                        now.now().format("%Y-%m-%d %H:%M:%S"),
                        record.target(),
                        msg
                    )?;
                    // Then add our helpful message
                    straico_proxy::tls_detector::log_https_error();
                    return Ok(());
                }
            }
            // Default formatting for other messages
            write!(
                w,
                "[{}] {} [{}] {}",
                now.now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                record.args()
            )
        });

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

    let http_addr = format!("{}:{}", cli.host, cli.port);
    let https_port = cli.https_port.unwrap_or(cli.port + 1);
    let https_addr = format!("{}:{}", cli.host, https_port);

    info!("Starting Straico proxy server...");
    info!("HTTP server running at http://{}", http_addr);
    info!("HTTPS rejection server running at https://{}", https_addr);
    info!("Completions endpoint: /v1/chat/completions");
    info!("\n┌─────────────────────────────────────────────────────────────────┐");
    info!("│ ✅ HTTPS connections now handled gracefully                      │");
    info!("│                                                                 │");
    info!("│ HTTPS clients will receive a proper error message explaining    │");
    info!("│ that only HTTP is supported.                                    │");
    info!("│                                                                 │");
    info!(
        "│ HTTP:  http://127.0.0.1:{}                                       │",
        cli.port
    );
    info!(
        "│ HTTPS: https://127.0.0.1:{} (returns error)                      │",
        https_port
    );
    info!("└─────────────────────────────────────────────────────────────────┘");

    let client = StraicoClient::builder()
        .pool_max_idle_per_host(25)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(90))
        .timeout(Duration::from_secs(90))
        .build()?;

    // Create TLS config for HTTPS rejection
    let tls_config = straico_proxy::https_rejector::create_self_signed_cert()?;

    let http_server = HttpServer::new(move || {
        let app_state = server::AppState {
            client: client.clone(),
            key: api_key.clone(),
            heartbeat_char: cli.heartbeat_char,
        };

        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(app_state))
            .service(server::openai_chat_completion)
            .service(server::model_handler)
            .service(server::models_handler)
            .default_service(web::to(HttpResponse::NotFound))
    });

    // Bind HTTP server
    let http_server = http_server
        .bind(&http_addr)
        .with_context(|| format!("Failed to bind HTTP to: {}", http_addr))?;

    // Create and bind HTTPS rejection server
    let https_server = HttpServer::new(|| {
        App::new().configure(straico_proxy::https_rejector::configure_https_rejector)
    })
    .bind_rustls_0_23(&https_addr, tls_config)
    .with_context(|| format!("Failed to bind HTTPS to: {}", https_addr))?;

    // Run both servers
    let http_handle = http_server.run();
    let https_handle = https_server.run();

    tokio::try_join!(http_handle, https_handle).context("Failed to run servers")?;

    Ok(())
}
