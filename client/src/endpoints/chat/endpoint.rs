use std::marker::PhantomData;

use reqwest::Method;
use serde::{de::DeserializeOwned, Serialize};

use crate::endpoints::{
    chat::{ChatRequest, ChatResponse},
    Endpoint,
};

/// The chat endpoint.
pub struct ChatEndpoint<T> {
    request: ChatRequest<T>,
    _phantom: PhantomData<T>,
}

impl<T: Serialize> ChatEndpoint<T> {
    /// Creates a new chat endpoint.
    pub fn new(request: ChatRequest<T>) -> Self {
        Self {
            request,
            _phantom: PhantomData,
        }
    }
}

impl<T: Serialize + Send + Sync> Endpoint for ChatEndpoint<T>
where
    ChatResponse<T>: DeserializeOwned,
{
    type Request = ChatRequest<T>;
    type Response = ChatResponse<T>;

    fn method(&self) -> Method {
        Method::POST
    }

    fn path(&self) -> &str {
        "/v2/chat/completions"
    }

    fn request_body(&self) -> &Self::Request {
        &self.request
    }
}
