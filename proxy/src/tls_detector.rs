use log::error;

/// Helper to format the startup message about HTTP-only support.
pub fn format_http_only_notice(port: u16) -> String {
    format!(
        "\n┌─────────────────────────────────────────────────────────────────┐\n\
         │ ⚠️  IMPORTANT: This server uses HTTP only (not HTTPS)          │\n\
         │                                                                 │\n\
         │ HTTPS/TLS attempts are detected and a helpful error message    │\n\
         │ is logged automatically. Make sure to use:                     │\n\
         │                                                                 │\n\
         │   http://127.0.0.1:{:<4} (not https://)                          │\n\
         └─────────────────────────────────────────────────────────────────┘",
        port
    )
}

/// Logs a helpful error when HTTP parse errors occur (likely due to TLS).
/// This is called by the custom log formatter when it detects "invalid Header" errors.
///
/// This approach is simpler and more reliable than trying to intercept at the TCP level,
/// as actix-web's architecture doesn't provide good hooks for peeking at streams before
/// the HTTP parser runs.
pub fn log_https_error() {
    error!("\n┌─────────────────────────────────────────────────────────────────┐");
    error!("│ ❌ HTTP Parse Error - Likely HTTPS/TLS to HTTP Server          │");
    error!("│                                                                 │");
    error!("│ Your client is probably using https:// instead of http://      │");
    error!("│ This server does NOT support HTTPS/TLS connections.            │");
    error!("│                                                                 │");
    error!("│ Fix: Change your client configuration to use http://           │");
    error!("└─────────────────────────────────────────────────────────────────┘");
}
