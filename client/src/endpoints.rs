pub mod chat;

use crate::endpoints::chat::chat_response::ChatResponse;
use serde::{Deserialize, Serialize};

/// Generic response wrapper for Straico API responses
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponseData {
    pub data: serde_json::Value,
}

impl ApiResponseData {
    /// Attempts to parse the response data as chat response data
    pub fn get_chat_response(self) -> Result<ChatResponse, serde_json::Error> {
        serde_json::from_value(self.data)
    }
}
