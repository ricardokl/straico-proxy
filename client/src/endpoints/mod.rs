pub mod chat;
pub mod completion;

use crate::endpoints::chat::chat_response::ChatResponse;
use crate::endpoints::completion::completion_response::CompletionData;
use serde::{Deserialize, Serialize};

/// Generic response wrapper for Straico API responses
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponseData {
    pub data: serde_json::Value,
}

impl ApiResponseData {
    /// Attempts to parse the response data as completion data
    pub fn get_completion(self) -> Result<CompletionData, serde_json::Error> {
        serde_json::from_value(self.data)
    }

    /// Attempts to parse the response data as chat response data
    pub fn get_chat_response(self) -> Result<ChatResponse, serde_json::Error> {
        serde_json::from_value(self.data)
    }
}
