use reqwest::Method;

use crate::endpoints::{
    models::{ChatModel, ModelsResponse},
    Endpoint,
};

/// An endpoint for listing all available models.
pub struct ListModelsEndpoint;

impl Endpoint for ListModelsEndpoint {
    type Request = ();
    type Response = ModelsResponse;

    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> &str {
        "/v2/models"
    }

    fn request_body(&self) -> &Self::Request {
        &()
    }
}

/// An endpoint for retrieving a single model by its ID.
pub struct GetModelEndpoint {
    path: String,
}

impl GetModelEndpoint {
    /// Creates a new endpoint for retrieving a single model.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            path: format!("/v2/models/{}", id.into()),
        }
    }
}

impl Endpoint for GetModelEndpoint {
    type Request = ();
    type Response = ChatModel;

    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn request_body(&self) -> &Self::Request {
        &()
    }
}
