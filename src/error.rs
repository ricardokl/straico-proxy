use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::Value;
use std::fmt::Debug;
use reqwest::Error as ReqwestError;
use thiserror::Error;

use anyhow::Error as AnyhowError;

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("Failed to serialize or deserialize JSON")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Error from HTTP client")]
    ReqwestClient(#[from] ReqwestError),
    #[error("Failed to parse response from Straico API")]
    ResponseParse(Value),
    #[error("An internal error occurred")]
    Anyhow(#[from] AnyhowError),
}

impl ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match *self {
            CustomError::SerdeJson(_) => StatusCode::BAD_REQUEST,
            CustomError::ReqwestClient(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::ResponseParse(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": {
                "message": self.to_string()
            }
        }))
    }
}
