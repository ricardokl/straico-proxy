use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatError {
    #[error("Failed to serialize tool calls: {0}")]
    ToolSerialization(#[from] serde_json::Error),
    #[error("Tool embedding error: {0}")]
    ToolEmbedding(String),
}
