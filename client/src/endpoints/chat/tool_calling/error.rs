use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolCallingError {
    #[error("Failed to serialize tool calls: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Tool embedding error: {0}")]
    Embedding(String),
}
