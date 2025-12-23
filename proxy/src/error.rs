use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use reqwest::Error as ReqwestError;
use serde_json::Value;
use std::fmt::Debug;
use straico_client::{ChatError, StraicoError};
use thiserror::Error;

use crate::streaming::create_error_chunk_with_type;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Failed to serialize or deserialize JSON")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Error from HTTP client")]
    ReqwestClient(#[from] ReqwestError),
    #[error("Error from Straico API")]
    Straico(#[from] StraicoError),
    #[error("Failed to parse response from Straico API")]
    ResponseParse(Value),
    #[error("Tool embedding error: {0}")]
    ToolEmbedding(String),
    #[error("Missing required field: {field}")]
    MissingRequiredField { field: String },
    #[error("Invalid parameter: {parameter} - {reason}")]
    InvalidParameter { parameter: String, reason: String },
    #[error("Chat error: {0}")]
    Chat(#[from] ChatError),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Rate limited: {message}")]
    RateLimited {
        retry_after: Option<u64>,
        message: String,
    },
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("Server configuration error: {0}")]
    ServerConfiguration(String),
    #[error("Upstream error: {1}")]
    UpstreamError(u16, String),
}

impl ProxyError {
    pub fn to_streaming_chunk(&self) -> Value {
        let message = match self {
            ProxyError::MissingRequiredField { field } => {
                format!("Missing required field: {field}")
            }
            ProxyError::InvalidParameter { parameter, reason } => {
                format!("Invalid parameter '{parameter}': {reason}")
            }
            ProxyError::ToolEmbedding(e) => format!("Tool error: {e}"),
            ProxyError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            ProxyError::ReqwestClient(e) => format!("Network error: {e}"),
            ProxyError::Straico(e) => format!("Upstream API error: {e}"),
            ProxyError::ResponseParse(_) => {
                "Failed to parse response from upstream API".to_string()
            }
            ProxyError::Chat(e) => format!("Chat processing error: {e}"),
            ProxyError::BadRequest(e) => format!("Bad request: {e}"),
            ProxyError::Unauthorized(msg) => format!("Unauthorized: {msg}"),
            ProxyError::Forbidden(msg) => format!("Forbidden: {msg}"),
            ProxyError::NotFound(msg) => format!("Not found: {msg}"),
            ProxyError::RateLimited {
                retry_after,
                message,
            } => {
                format!(
                    "Rate limited: {message}{}",
                    retry_after
                        .map(|s| format!(" (retry after {} seconds)", s))
                        .unwrap_or_default()
                )
            }
            ProxyError::ServiceUnavailable(msg) => {
                format!("Service unavailable: {msg}")
            }
            ProxyError::ServerConfiguration(msg) => {
                format!("Server configuration error: {msg}")
            }
            ProxyError::UpstreamError(_, msg) => {
                format!("Upstream error: {msg}")
            }
        };
        create_error_chunk_with_type(&message, self.error_type(), self.error_code())
    }

    /// Maps the error to an appropriate OpenAI-compatible error type
    pub fn error_type(&self) -> &'static str {
        match self {
            ProxyError::SerdeJson(_) => "invalid_request_error",
            ProxyError::ReqwestClient(_) => "api_error",
            ProxyError::Straico(_) => "api_error",
            ProxyError::ResponseParse(_) => "api_error",
            ProxyError::ToolEmbedding(_) => "invalid_request_error",
            ProxyError::MissingRequiredField { .. } => "invalid_request_error",
            ProxyError::InvalidParameter { .. } => "invalid_request_error",
            ProxyError::Chat(_) => "invalid_request_error",
            ProxyError::BadRequest(_) => "invalid_request_error",
            ProxyError::Unauthorized(_) => "authentication_error",
            ProxyError::Forbidden(_) => "permission_error",
            ProxyError::NotFound(_) => "invalid_request_error",
            ProxyError::RateLimited { .. } => "rate_limit_error",
            ProxyError::ServiceUnavailable(_) => "api_error",
            ProxyError::ServerConfiguration(_) => "server_error",
            ProxyError::UpstreamError(_, _) => "api_error",
        }
    }

    /// Maps the error to an appropriate OpenAI-compatible error code
    pub fn error_code(&self) -> Option<&'static str> {
        match self {
            ProxyError::SerdeJson(_) => Some("invalid_json"),
            ProxyError::ReqwestClient(_) => Some("network_error"),
            ProxyError::Straico(_) => Some("upstream_error"),
            ProxyError::ResponseParse(_) => Some("response_parse_error"),
            ProxyError::ToolEmbedding(_) => Some("tool_error"),
            ProxyError::MissingRequiredField { .. } => Some("missing_field"),
            ProxyError::InvalidParameter { .. } => Some("invalid_parameter"),
            ProxyError::Chat(_) => Some("chat_error"),
            ProxyError::BadRequest(_) => Some("bad_request"),
            ProxyError::Unauthorized(_) => Some("unauthorized"),
            ProxyError::Forbidden(_) => Some("forbidden"),
            ProxyError::NotFound(_) => Some("not_found"),
            ProxyError::RateLimited { .. } => Some("rate_limit_exceeded"),
            ProxyError::ServiceUnavailable(_) => Some("service_unavailable"),
            ProxyError::ServerConfiguration(_) => Some("server_configuration"),
            ProxyError::UpstreamError(_, _) => Some("upstream_error"),
        }
    }
}

impl ResponseError for ProxyError {
    fn status_code(&self) -> StatusCode {
        match self {
            ProxyError::SerdeJson(_) => StatusCode::BAD_REQUEST,
            ProxyError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ProxyError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ProxyError::Forbidden(_) => StatusCode::FORBIDDEN,
            ProxyError::NotFound(_) => StatusCode::NOT_FOUND,
            ProxyError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            ProxyError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ProxyError::ServerConfiguration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::UpstreamError(status, _) => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            ProxyError::ReqwestClient(e) => {
                // Return specific status codes based on the reqwest error type
                if e.is_timeout() {
                    StatusCode::GATEWAY_TIMEOUT
                } else if e.is_connect() {
                    StatusCode::BAD_GATEWAY
                } else if let Some(status) = e.status() {
                    // Convert reqwest::StatusCode to actix_web::http::StatusCode
                    StatusCode::from_u16(status.as_u16())
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            }
            ProxyError::Straico(e) => {
                // Try to extract status code from StraicoError if it wraps a reqwest error
                match e {
                    straico_client::StraicoError::Request(req_err) => {
                        if req_err.is_timeout() {
                            StatusCode::GATEWAY_TIMEOUT
                        } else if req_err.is_connect() {
                            StatusCode::BAD_GATEWAY
                        } else if let Some(status) = req_err.status() {
                            // Convert reqwest::StatusCode to actix_web::http::StatusCode
                            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY)
                        } else {
                            StatusCode::BAD_GATEWAY
                        }
                    }
                    _ => StatusCode::BAD_GATEWAY,
                }
            }
            ProxyError::ResponseParse(_) => StatusCode::BAD_GATEWAY,
            ProxyError::ToolEmbedding(_) => StatusCode::BAD_REQUEST,
            ProxyError::MissingRequiredField { .. } => StatusCode::BAD_REQUEST,
            ProxyError::InvalidParameter { .. } => StatusCode::BAD_REQUEST,
            ProxyError::Chat(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_message = match self {
            ProxyError::MissingRequiredField { field } => {
                format!("Missing required field: {field}")
            }
            ProxyError::InvalidParameter { parameter, reason } => {
                format!("Invalid parameter '{parameter}': {reason}")
            }
            ProxyError::ToolEmbedding(e) => format!("Tool error: {e}"),
            ProxyError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            ProxyError::ReqwestClient(e) => format!("Network error: {e}"),
            ProxyError::Straico(e) => format!("Upstream API error: {e}"),
            ProxyError::ResponseParse(_) => {
                "Failed to parse response from upstream API".to_string()
            }
            ProxyError::Chat(e) => format!("Chat processing error: {e}"),
            ProxyError::BadRequest(e) => format!("Bad request: {e}"),
            ProxyError::Unauthorized(msg) => format!("Unauthorized: {msg}"),
            ProxyError::Forbidden(msg) => format!("Forbidden: {msg}"),
            ProxyError::NotFound(msg) => format!("Not found: {msg}"),
            ProxyError::RateLimited {
                retry_after,
                message,
            } => {
                format!(
                    "Rate limited: {message}{}",
                    retry_after
                        .map(|s| format!(" (retry after {} seconds)", s))
                        .unwrap_or_default()
                )
            }
            ProxyError::ServiceUnavailable(msg) => {
                format!("Service unavailable: {msg}")
            }
            ProxyError::ServerConfiguration(msg) => {
                format!("Server configuration error: {msg}")
            }
            ProxyError::UpstreamError(_, msg) => {
                format!("Upstream error: {msg}")
            }
        };

        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": {
                "message": error_message,
                "type": self.error_type(),
                "code": self.error_code()
            }
        }))
    }
}
