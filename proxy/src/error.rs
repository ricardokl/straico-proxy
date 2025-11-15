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
            CustomError::MissingRequiredField { field } => format!("Missing required field: {field}"),
            CustomError::InvalidParameter { parameter, reason } => {
                format!("Invalid parameter '{parameter}': {reason}")
            }
            CustomError::ToolEmbedding(e) => format!("Tool error: {e}"),
            CustomError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            CustomError::ReqwestClient(e) => format!("Network error: {e}"),
            CustomError::Straico(e) => format!("Upstream API error: {e}"),
            CustomError::ResponseParse(_) => "Failed to parse response from upstream API".to_string(),
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
                    StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
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
            CustomError::MissingRequiredField { field } => format!("Missing required field: {field}"),
            CustomError::InvalidParameter { parameter, reason } => {
                format!("Invalid parameter '{parameter}': {reason}")
            }
            CustomError::ToolEmbedding(e) => format!("Tool error: {e}"),
            CustomError::SerdeJson(e) => format!("Invalid JSON: {e}"),
            CustomError::ReqwestClient(e) => format!("Network error: {e}"),
            CustomError::Straico(e) => format!("Upstream API error: {e}"),
            CustomError::ResponseParse(_) => "Failed to parse response from upstream API".to_string(),
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
            (CustomError::MissingRequiredField { field: "test".to_string() }, "invalid_request_error"),
            (CustomError::InvalidParameter { parameter: "test".to_string(), reason: "invalid".to_string() }, "invalid_request_error"),
            (CustomError::ToolEmbedding("test".to_string()), "invalid_request_error"),
            (CustomError::ResponseParse(serde_json::json!({})), "api_error"),
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
            (CustomError::MissingRequiredField { field: "test".to_string() }, Some("missing_field")),
            (CustomError::InvalidParameter { parameter: "test".to_string(), reason: "invalid".to_string() }, Some("invalid_parameter")),
            (CustomError::ToolEmbedding("test".to_string()), Some("tool_error")),
            (CustomError::ResponseParse(serde_json::json!({})), Some("response_parse_error")),
        ];

        for (error, expected_code) in errors {
            assert_eq!(error.error_code(), expected_code);
        }
    }

    #[test]
    fn test_error_response_format() {
        let error = CustomError::MissingRequiredField { field: "model".to_string() };
        let response = error.error_response();

        // Check status code
        assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);

        // The response body would need to be extracted and parsed to test JSON structure
        // This is more complex in actix-web, so we'll test the streaming chunk format instead
    }

    #[test]
    fn test_streaming_chunk_format() {
        let error = CustomError::InvalidParameter { 
            parameter: "temperature".to_string(), 
            reason: "must be between 0 and 2".to_string() 
        };
        let chunk = error.to_streaming_chunk();

        assert_eq!(chunk["error"]["message"], "Invalid parameter 'temperature': must be between 0 and 2");
        assert_eq!(chunk["error"]["type"], "invalid_request_error");
        assert_eq!(chunk["error"]["code"], "invalid_parameter");
    }

    #[test]
    fn test_different_error_types_have_different_formats() {
        // Create a serde_json error by trying to parse invalid JSON
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();

        let errors = vec![
            CustomError::SerdeJson(json_error),
            CustomError::MissingRequiredField { field: "model".to_string() },
            CustomError::InvalidParameter { parameter: "temperature".to_string(), reason: "invalid".to_string() },
            CustomError::ToolEmbedding("tool parse error".to_string()),
            CustomError::ResponseParse(serde_json::json!({})),
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
        assert!(error_types.len() > 1, "Expected multiple error types, got: {:?}", error_types);
        assert!(error_codes.len() > 1, "Expected multiple error codes, got: {:?}", error_codes);
    }

    #[test]
    fn test_reqwest_error_status_codes() {
        use actix_web::http::StatusCode;

        // Test timeout error returns GATEWAY_TIMEOUT
        let timeout_url = "http://example.com:81"; // Non-responsive port
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let timeout_result = rt.block_on(async {
            client.get(timeout_url).send().await
        });
        
        if let Err(e) = timeout_result {
            if e.is_timeout() {
                let error = CustomError::ReqwestClient(e);
                assert_eq!(error.status_code(), StatusCode::GATEWAY_TIMEOUT);
            }
        }
    }

    #[test]
    fn test_response_parse_error_status_code() {
        let error = CustomError::ResponseParse(serde_json::json!({"error": "test"}));
        assert_eq!(error.status_code(), actix_web::http::StatusCode::BAD_GATEWAY);
    }
}
