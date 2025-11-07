use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use reqwest::Error as ReqwestError;
use serde_json::Value;
use std::fmt::Debug;
use straico_client::{ChatError, StraicoError};
use thiserror::Error;

use crate::streaming::create_error_chunk_with_type;
use anyhow::Error as AnyhowError;

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
    #[error("An internal error occurred")]
    Anyhow(#[from] AnyhowError),
    #[error("Tool embedding error: {0}")]
    ToolEmbedding(String),
    #[error("Request validation error: {0}")]
    RequestValidation(String),
    #[error("Chat error: {0}")]
    Chat(#[from] ChatError),
}

impl From<String> for CustomError {
    fn from(s: String) -> Self {
        CustomError::RequestValidation(s)
    }
}

impl CustomError {
    pub fn to_streaming_chunk(&self) -> Value {
        let message = match self {
            CustomError::RequestValidation(e) => format!("Invalid request: {e}"),
            CustomError::ToolEmbedding(e) => format!("Tool error: {e}"),
            CustomError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            CustomError::ReqwestClient(e) => format!("Network error: {e}"),
            CustomError::Straico(e) => format!("Upstream API error: {e}"),
            CustomError::ResponseParse(_) => "Failed to parse response from upstream API".to_string(),
            CustomError::Chat(e) => format!("Chat processing error: {e}"),
            CustomError::Anyhow(e) => format!("Internal server error: {e}"),
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
            CustomError::Anyhow(_) => "server_error",
            CustomError::ToolEmbedding(_) => "invalid_request_error",
            CustomError::RequestValidation(_) => "invalid_request_error",
            CustomError::Chat(_) => "invalid_request_error",
        }
    }

    /// Maps the error to an appropriate OpenAI-compatible error code
    pub fn error_code(&self) -> Option<&'static str> {
        match self {
            CustomError::SerdeJson(_) => Some("invalid_json"),
            CustomError::ReqwestClient(_) => Some("network_error"),
            CustomError::Straico(_) => Some("upstream_error"),
            CustomError::ResponseParse(_) => Some("response_parse_error"),
            CustomError::Anyhow(_) => Some("internal_error"),
            CustomError::ToolEmbedding(_) => Some("tool_error"),
            CustomError::RequestValidation(_) => Some("invalid_parameter"),
            CustomError::Chat(_) => Some("chat_error"),
        }
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match *self {
            CustomError::SerdeJson(_) => StatusCode::BAD_REQUEST,
            CustomError::ReqwestClient(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::Straico(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::ResponseParse(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::ToolEmbedding(_) => StatusCode::BAD_REQUEST,
            CustomError::RequestValidation(_) => StatusCode::BAD_REQUEST,
            CustomError::Chat(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_message = match self {
            CustomError::RequestValidation(e) => format!("Invalid request: {e}"),
            CustomError::ToolEmbedding(e) => format!("Tool error: {e}"),
            CustomError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            CustomError::ReqwestClient(e) => format!("Network error: {e}"),
            CustomError::Straico(e) => format!("Upstream API error: {e}"),
            CustomError::ResponseParse(_) => "Failed to parse response from upstream API".to_string(),
            CustomError::Chat(e) => format!("Chat processing error: {e}"),
            CustomError::Anyhow(e) => format!("Internal server error: {e}"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::ResponseError;

    #[test]
    fn test_error_type_mapping() {
        // Create a serde_json error by trying to parse invalid JSON
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();

        let errors = vec![
            (CustomError::SerdeJson(json_error), "invalid_request_error"),
            (CustomError::RequestValidation("test".to_string()), "invalid_request_error"),
            (CustomError::ToolEmbedding("test".to_string()), "invalid_request_error"),
            (CustomError::ResponseParse(serde_json::json!({})), "api_error"),
            (CustomError::Anyhow(anyhow::anyhow!("test")), "server_error"),
        ];

        for (error, expected_type) in errors {
            assert_eq!(error.error_type(), expected_type);
        }
    }

    #[test]
    fn test_error_code_mapping() {
        // Create a serde_json error by trying to parse invalid JSON
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();

        let errors = vec![
            (CustomError::SerdeJson(json_error), Some("invalid_json")),
            (CustomError::RequestValidation("test".to_string()), Some("invalid_parameter")),
            (CustomError::ToolEmbedding("test".to_string()), Some("tool_error")),
            (CustomError::ResponseParse(serde_json::json!({})), Some("response_parse_error")),
            (CustomError::Anyhow(anyhow::anyhow!("test")), Some("internal_error")),
        ];

        for (error, expected_code) in errors {
            assert_eq!(error.error_code(), expected_code);
        }
    }

    #[test]
    fn test_error_response_format() {
        let error = CustomError::RequestValidation("Missing required field 'model'".to_string());
        let response = error.error_response();

        // Check status code
        assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);

        // The response body would need to be extracted and parsed to test JSON structure
        // This is more complex in actix-web, so we'll test the streaming chunk format instead
    }

    #[test]
    fn test_streaming_chunk_format() {
        let error = CustomError::RequestValidation("Invalid parameter".to_string());
        let chunk = error.to_streaming_chunk();

        assert_eq!(chunk["error"]["message"], "Invalid request: Invalid parameter");
        assert_eq!(chunk["error"]["type"], "invalid_request_error");
        assert_eq!(chunk["error"]["code"], "invalid_parameter");
    }

    #[test]
    fn test_different_error_types_have_different_formats() {
        // Create a serde_json error by trying to parse invalid JSON
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();

        let errors = vec![
            CustomError::SerdeJson(json_error),
            CustomError::RequestValidation("missing field".to_string()),
            CustomError::ToolEmbedding("tool parse error".to_string()),
            CustomError::Anyhow(anyhow::anyhow!("internal error")),
        ];

        let mut error_types = std::collections::HashSet::new();
        let mut error_codes = std::collections::HashSet::new();

        for error in errors {
            error_types.insert(error.error_type());
            if let Some(code) = error.error_code() {
                error_codes.insert(code);
            }
        }

        // Should have different error types and codes
        assert!(error_types.len() > 1);
        assert!(error_codes.len() > 1);
    }
}
