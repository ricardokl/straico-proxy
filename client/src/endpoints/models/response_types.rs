use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<Model>,
}

#[derive(Debug, Deserialize)]
pub struct Model {
    pub name: String,
    pub id: String,
    pub word_limit: i64,
    pub pricing: Pricing,
    pub max_output: i64,
    pub metadata: Metadata,
    pub owned_by: String,
    pub created: i64,
    pub object: String,
    pub model_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Pricing {
    pub coins: f64,
    pub words: i64,
}

#[derive(Debug, Deserialize)]
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
