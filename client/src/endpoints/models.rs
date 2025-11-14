pub mod response_types;
pub use response_types::*;

use crate::client::{ApiKeySet, NoApiKey, StraicoClient, StraicoRequestBuilder};

impl StraicoClient {
    pub fn models(self) -> StraicoRequestBuilder<NoApiKey, ()> {
        let url = self
            .base_url
            .unwrap_or_else(|| "https://api.straico.com".to_string())
            + "/v2/models";
        self.client.get(&url).into()
    }
}

impl StraicoRequestBuilder<ApiKeySet, ()> {
    pub async fn send_and_parse(self) -> Result<ModelsResponse, reqwest::Error> {
        self.0.send().await?.json().await
    }
}
