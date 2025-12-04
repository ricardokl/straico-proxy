use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use reqwest::Error as ReqwestError;
use serde_json::Value;
use std::fmt::Debug;
use straico_client::{ChatError, StraicoError};
use thiserror::Error;

use crate::streaming::create_error_chunk_with_type;

#[derive(Error, Debug)]
pub enum CustomError {
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

impl CustomError {
    pub fn to_streaming_chunk(&self) -> Value {
        let message = match self {
            CustomError::MissingRequiredField { field } => {
                format!("Missing required field: {field}")
            }
            CustomError::InvalidParameter { parameter, reason } => {
                format!("Invalid parameter '{parameter}': {reason}")
            }
            CustomError::ToolEmbedding(e) => format!("Tool error: {e}"),
            CustomError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            CustomError::ReqwestClient(e) => format!("Network error: {e}"),
            CustomError::Straico(e) => format!("Upstream API error: {e}"),
            CustomError::ResponseParse(_) => {
                "Failed to parse response from upstream API".to_string()
            }
            CustomError::Chat(e) => format!("Chat processing error: {e}"),
            CustomError::BadRequest(e) => format!("Bad request: {e}"),
        };
        create_error_chunk_with_type(&message, self.error_type(), self.error_code())
    }

    /// Maps the error to an appropriate OpenAI-compatible error type
    pub fn error_type(&self) -> &'static str {
        match self {
            CustomError::SerdeJson(_) => "invalid_request_error",
            CustomError::ReqwestClient(_) => "api_error",
            CustomError::Straico(_) => "api_error",
            CustomError::ResponseParse(_) => "api_error",
            CustomError::ToolEmbedding(_) => "invalid_request_error",
            CustomError::MissingRequiredField { .. } => "invalid_request_error",
            CustomError::InvalidParameter { .. } => "invalid_request_error",
            CustomError::Chat(_) => "invalid_request_error",
            CustomError::BadRequest(_) => "invalid_request_error",
        }
    }

    /// Maps the error to an appropriate OpenAI-compatible error code
    pub fn error_code(&self) -> Option<&'static str> {
        match self {
            CustomError::SerdeJson(_) => Some("invalid_json"),
            CustomError::ReqwestClient(_) => Some("network_error"),
            CustomError::Straico(_) => Some("upstream_error"),
            CustomError::ResponseParse(_) => Some("response_parse_error"),
            CustomError::ToolEmbedding(_) => Some("tool_error"),
            CustomError::MissingRequiredField { .. } => Some("missing_field"),
            CustomError::InvalidParameter { .. } => Some("invalid_parameter"),
            CustomError::Chat(_) => Some("chat_error"),
            CustomError::BadRequest(_) => Some("bad_request"),
        }
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match self {
            CustomError::SerdeJson(_) => StatusCode::BAD_REQUEST,
            CustomError::BadRequest(_) => StatusCode::BAD_REQUEST,
            CustomError::ReqwestClient(e) => {
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
            CustomError::Straico(e) => {
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
            CustomError::ResponseParse(_) => StatusCode::BAD_GATEWAY,
            CustomError::ToolEmbedding(_) => StatusCode::BAD_REQUEST,
            CustomError::MissingRequiredField { .. } => StatusCode::BAD_REQUEST,
            CustomError::InvalidParameter { .. } => StatusCode::BAD_REQUEST,
            CustomError::Chat(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_message = match self {
            CustomError::MissingRequiredField { field } => {
                format!("Missing required field: {field}")
            }
            CustomError::InvalidParameter { parameter, reason } => {
                format!("Invalid parameter '{parameter}': {reason}")
            }
            CustomError::ToolEmbedding(e) => format!("Tool error: {e}"),
            CustomError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            CustomError::ReqwestClient(e) => format!("Network error: {e}"),
            CustomError::Straico(e) => format!("Upstream API error: {e}"),
            CustomError::ResponseParse(_) => {
                "Failed to parse response from upstream API".to_string()
            }
            CustomError::Chat(e) => format!("Chat processing error: {e}"),
            CustomError::BadRequest(e) => format!("Bad request: {e}"),
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
