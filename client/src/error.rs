use thiserror::Error;

/// A custom error type for the Straico API client.
#[derive(Error, Debug)]
pub enum StraicoError {
    /// An error occurred while making a request.
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
    /// An error occurred while serializing or deserializing data.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    /// An error returned by the API.
    #[error("api error: {0}")]
    Api(String),
    /// An error occurred while performing a regex operation.
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
}
