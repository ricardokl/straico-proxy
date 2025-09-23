use actix_web::{App, HttpResponse, HttpServer, web};
use anyhow::Context;
use clap::Parser;
use flexi_logger::{FileSpec, Logger, WriteMode};
use log::{info, warn};
use straico_client::client::StraicoClient;
use straico_proxy::{
    cli::Cli,
    config::ProxyConfig,
    config_manager::ConfigManager,
    server,
};



#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle config file creation
    if cli.create_config {
        if let Err(e) = ConfigManager::create_default_config(&cli.config) {
            return Err(anyhow::anyhow!("Failed to create config file: {}", e));
        }
        info!("Created default configuration file: {}", cli.config);
        return Ok(());
    }

    // Load and merge configuration
    let mut config_manager = ConfigManager::new(&cli.config);
    config_manager.merge_with_cli_args(&cli);

    // Apply feature flags from CLI
    for feature in &cli.enable_feature {
        if let Err(e) = config_manager.set_feature_flag(feature, true) {
            warn!("Failed to enable feature '{}': {}", feature, e);
        }
    }
    for feature in &cli.disable_feature {
        if let Err(e) = config_manager.set_feature_flag(feature, false) {
            warn!("Failed to disable feature '{}': {}", feature, e);
        }
    }

    // Validate configuration
    if let Err(errors) = config_manager.validate_config() {
        for error in errors {
            warn!("Configuration error: {}", error);
        }
        return Err(anyhow::anyhow!("Configuration validation failed"));
    }

    let effective_config = config_manager.get_effective_config();

    // Set up logging
    let log_level = cli.log_level
        .as_deref()
        .or(Some(effective_config.environment.log_level.as_str()))
        .unwrap_or({
            if cli.print_request_raw
                || cli.print_request_converted
                || cli.print_response_raw
                || cli.print_response_converted
            {
                "debug"
            } else {
                "info"
            }
        });

    let mut logger = Logger::try_with_str(log_level)?.write_mode(WriteMode::BufferAndFlush);

    if cli.log_to_file || effective_config.features.enable_request_logging {
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

    let host = cli.environment.as_deref()
        .map(|_| effective_config.environment.host.clone())
        .unwrap_or(cli.host);
    let port = effective_config.environment.port;
    
    let addr = format!("{}:{}", host, port);
    info!("Starting Straico proxy server...");
    info!("Environment: {}", effective_config.environment.environment);
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

    // Display configuration summary
    info!("Configuration Summary:");
    info!("  - Environment: {}", effective_config.environment.environment);
    info!("  - Use new chat endpoint: {}", effective_config.proxy.use_new_chat_endpoint);
    info!("  - Validate requests: {}", effective_config.proxy.validate_requests);
    info!("  - Include debug info: {}", effective_config.proxy.include_debug_info);
    info!("  - Enabled features: {:?}", effective_config.get_enabled_features());

    // Apply configuration limits
    let mut proxy_config = effective_config.proxy.clone();
    if let Some(max_messages) = cli.max_messages {
        proxy_config.max_messages_per_request = Some(max_messages);
    }
    if let Some(max_content) = cli.max_content_length {
        proxy_config.max_content_length = Some(max_content);
    }

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server::AppState {
                client: StraicoClient::new(),
                key: api_key.clone(),
                config: proxy_config.clone(),
                print_request_raw: cli.print_request_raw,
                print_request_converted: cli.print_request_converted,
                print_response_raw: cli.print_response_raw,
                print_response_converted: cli.print_response_converted,
                use_new_chat_endpoint: effective_config.proxy.use_new_chat_endpoint,
                force_new_endpoint_for_tools: effective_config.proxy.force_new_endpoint_for_tools,
            }))
            .service(server::openai_chat_completion)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .bind(addr)?
    .run()
    .await
    .context("Failed to run HTTP server")
}
