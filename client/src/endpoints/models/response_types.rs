use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Response type for listing models via `GET /v2/models`.
#[derive(Debug, Deserialize, Serialize)]
pub struct ModelsResponse {
    /// Collection of models returned by the list endpoint.
    pub data: Vec<ChatModel>,
    /// Optional success flag returned by the API (present in v2).
    #[serde(default)]
    pub success: Option<bool>,
}

/// Response type for retrieving a single model via `GET /v2/models/{model_id}`.
///
/// The inner `data` object uses the same `ChatModel` representation as the list
/// endpoint, and additional fields returned by the API are ignored.
#[derive(Debug, Deserialize, Serialize)]
pub struct ModelResponse {
    pub data: ChatModel,
    #[serde(default)]
    pub success: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatModel {
    pub name: String,
    /// Model identifier. In v1 this field was called `model`, in v2 it is `id`.
    /// Accept both for backward compatibility.
    #[serde(alias = "model")]
    pub id: String,
    #[serde(default)]
    pub word_limit: Option<i64>,
    /// Raw pricing information as returned by the API.
    ///
    /// Different model types (chat, image, audio, etc.) expose different
    /// pricing structures, so we keep this as unstructured JSON rather than
    /// assuming a fixed set of keys.
    pub pricing: JsonValue,
    #[serde(default)]
    pub max_output: Option<i64>,
    #[serde(default)]
    pub metadata: Option<Metadata>,
    #[serde(default)]
    pub owned_by: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub model_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    #[serde(default)]
    pub editors_link: String,
    #[serde(default)]
    pub editors_choice_level: i64,
    #[serde(default)]
    pub cons: Vec<String>,
    #[serde(default)]
    pub pros: Vec<String>,
    #[serde(default)]
    pub applications: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub other: Vec<String>,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub model_date: String,
}
