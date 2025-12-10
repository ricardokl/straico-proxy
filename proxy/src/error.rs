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
        }
    }
}

impl ResponseError for ProxyError {
    fn status_code(&self) -> StatusCode {
        match self {
            ProxyError::SerdeJson(_) => StatusCode::BAD_REQUEST,
            ProxyError::BadRequest(_) => StatusCode::BAD_REQUEST,
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