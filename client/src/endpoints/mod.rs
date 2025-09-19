pub mod chat;
pub mod completion;

use serde::{Deserialize, Serialize};
use crate::endpoints::completion::completion_response::CompletionData;

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
}