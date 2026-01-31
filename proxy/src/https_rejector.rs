use actix_web::{web, HttpRequest, HttpResponse};
use log::{info, warn};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::io::{self, BufReader};

/// Handler that rejects all HTTPS requests with a helpful message
pub async fn reject_https(_req: HttpRequest) -> HttpResponse {
    warn!("HTTPS request received - returning rejection message");

    HttpResponse::BadRequest()
        .content_type("text/plain; charset=utf-8")
        .body(concat!(
            "HTTPS/TLS Not Supported\n",
            "========================\n\n",
            "This server does not support HTTPS connections.\n",
            "Please reconfigure your client to use plain HTTP instead.\n\n",
            "Change: https://127.0.0.1:PORT\n",
            "To:     http://127.0.0.1:PORT\n\n",
            "The HTTP-only port is typically one less than this HTTPS port.\n"
        ))
}

/// Creates a self-signed certificate for HTTPS rejection purposes only.
/// This allows us to complete the TLS handshake so we can send a proper HTTP error.
pub fn create_self_signed_cert() -> io::Result<ServerConfig> {
    info!("Generating self-signed certificate for HTTPS rejection...");

    // Use rcgen to generate a self-signed certificate
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];

    let cert = rcgen::generate_simple_self_signed(subject_alt_names).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to generate cert: {}", e),
        )
    })?;

    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    // Parse the certificate and private key
    let cert_chain = certs(&mut BufReader::new(cert_pem.as_bytes()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid cert: {}", e)))?;

    let mut keys = pkcs8_private_keys(&mut BufReader::new(key_pem.as_bytes()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid key: {}", e)))?;

    if keys.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No private key found",
        ));
    }

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0).into())
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid TLS config: {}", e),
            )
        })?;

    Ok(config)
}

/// Configures the app for HTTPS rejection
pub fn configure_https_rejector(cfg: &mut web::ServiceConfig) {
    cfg.default_service(web::to(reject_https));
}
