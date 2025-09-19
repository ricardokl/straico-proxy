pub mod chat;
pub mod completion;

use serde::{Deserialize, Serialize};

/// Generic response wrapper for Straico API responses
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponseData {
    pub data: serde_json::Value,
}