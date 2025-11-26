use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ModelsResponse {
	pub data: Vec<ChatModel>,
	/// Optional success flag returned by the API (present in v2)
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
    pub pricing: Pricing,
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
pub struct Pricing {
    pub coins: f64,
    pub words: i64,
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
