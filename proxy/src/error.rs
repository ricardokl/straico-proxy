use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::Value;
use std::fmt::Debug;
use reqwest::Error as ReqwestError;
use thiserror::Error;
use straico_client::error::StraicoError;

use crate::streaming::create_error_chunk;
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
    ToolEmbedding(#[from] crate::tool_embedding::ToolEmbeddingError),
    #[error("Request validation error: {0}")]
    RequestValidation(String),
}

impl From<String> for CustomError {
    fn from(s: String) -> Self {
        CustomError::RequestValidation(s)
    }
}

impl CustomError {
    pub fn to_streaming_chunk(&self) -> Value {
        create_error_chunk(&self.to_string())
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
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_message = match self {
            CustomError::ToolEmbedding(e) => format!("Tool processing failed: {}", e),
            CustomError::RequestValidation(e) => format!("Invalid request: {}", e),
            _ => self.to_string(),
        };

        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": {
                "message": error_message,
                "type": "invalid_request_error",
                "code": null
            }
        }))
    }
}
