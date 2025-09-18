pub mod completion;

use crate::error::StraicoError;
use completion::completion_response::CompletionData;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiResponseData {
    success: bool,
    #[serde(flatten)]
    response: ApiResponseVariant,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum ApiResponseVariant {
    Error { error: String },
    Data { data: CompletionData },
}

impl ApiResponseData {
    pub fn get_completion(
        self,
    ) -> Result<completion::completion_response::Completion, StraicoError> {
        match self.response {
            ApiResponseVariant::Data { data } => Ok(data.get_completion_data()),
            ApiResponseVariant::Error { error } => Err(StraicoError::Api(error)),
        }
    }
}
