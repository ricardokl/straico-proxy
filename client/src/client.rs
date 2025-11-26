use reqwest::{Client, RequestBuilder};
use serde::Serialize;
use std::{fmt::Display, future::Future, marker::PhantomData};

use crate::endpoints::chat::{ChatMessage, ChatRequest};

/// Represents the state where no API key has been set for the request
pub struct NoApiKey;
/// Represents the state where an API key has been set for the request
pub struct ApiKeySet;
/// Represents the state where a payload has been set for the request
pub struct PayloadSet;

/// Builder for making requests to Straico API endpoints
///
/// # Type Parameters
///
/// * `Api` - Represents the authentication state (NoApiKey or ApiKeySet)
/// * `Payload` - Represents the request payload state
/// * `Response` - The expected response type from the API
//pub struct StraicoRequestBuilder<Api, Payload, Response>(
pub struct StraicoRequestBuilder<Api, Payload>(
    pub RequestBuilder,
    pub PhantomData<Payload>,
    pub PhantomData<Api>,
);

impl From<Client> for StraicoClient {
    fn from(value: Client) -> Self {
        Self {
            client: value,
            base_url: None,
        }
    }
}

/// A client for interacting with the Straico API
///
/// Wraps a reqwest::Client and provides convenient methods for making API requests.
/// Can be created using `StraicoClient::new()` or by converting a reqwest::Client
/// using `Into<StraicoClient>`.
#[derive(Clone)]
pub struct StraicoClient {
    pub client: Client,
    pub base_url: Option<String>,
}

impl Default for StraicoClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
            base_url: None,
        }
    }
}

impl StraicoClient {
    /// Creates a new instance of StraicoClient with default configuration
    ///
    /// This is a convenience constructor that creates a new reqwest::Client with default settings
    /// and converts it into a StraicoClient.
    ///
    /// # Returns
    ///
    /// A new StraicoClient instance ready to make API requests
    pub fn new() -> StraicoClient {
        StraicoClient::default()
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url: Some(base_url),
        }
    }

    /// Creates a request builder for the new chat endpoint
    ///
    /// # Returns
    ///
    /// A `StraicoRequestBuilder` configured for making chat completion requests
    pub fn chat(self) -> StraicoRequestBuilder<NoApiKey, ChatRequest<ChatMessage>> {
        let url = self
            .base_url
            .unwrap_or_else(|| "https://api.straico.com".to_string())
            + "/v2/chat/completions";
        self.client.post(&url).into()
    }

    /// Creates a request builder for the models endpoint
    ///
    /// # Returns
    ///
    /// A `StraicoRequestBuilder` configured for making a models request. The response
    /// can be sent using [send](StraicoRequestBuilder::send).
    pub fn models(self) -> StraicoRequestBuilder<NoApiKey, ()> {
        let url = self
            .base_url
            .unwrap_or_else(|| "https://api.straico.com".to_string())
            + "/v2/models";
        self.client.get(&url).into()
    }
}

impl<T> StraicoRequestBuilder<NoApiKey, T> {
    /// Sets the Bearer authentication token (API key) for this request
    ///
    /// # Arguments
    ///
    /// * `api_key` - The API key to use for authentication. Must implement Display trait.
    ///
    /// # Returns
    ///
    /// A new StraicoRequestBuilder with the ApiKeySet state, preserving the payload and response types
    pub fn bearer_auth<K: Display>(self, api_key: K) -> StraicoRequestBuilder<ApiKeySet, T> {
        self.0.bearer_auth(api_key).into()
    }
}

impl<K, T: Serialize> StraicoRequestBuilder<K, T> {
    /// Sets the JSON payload for the request
    ///
    /// # Arguments
    ///
    /// * `payload` - The payload to serialize as JSON. Must implement Into<T> where T is the expected payload type.
    ///
    /// # Returns
    ///
    /// A new StraicoRequestBuilder with the PayloadSet state, preserving the API key and response types
    pub fn json(self, payload: T) -> StraicoRequestBuilder<K, PayloadSet> {
        self.0.json(&payload).into()
    }
}

impl<T> StraicoRequestBuilder<ApiKeySet, T> {
    /// Sends the configured request to the API and returns the raw response
    ///
    /// This method will send the HTTP request that has been configured with authentication
    /// and payload (if applicable).
    ///
    /// # Returns
    ///
    /// A Future that resolves to a Result containing either:
    /// - The raw `reqwest::Response`
    /// - A reqwest error if the request fails
    pub fn send(self) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> {
        self.0.send()
    }
}

impl<T, U> From<RequestBuilder> for StraicoRequestBuilder<T, U> {
    /// Converts a RequestBuilder into a StraicoRequestBuilder
    ///
    /// This implementation allows for easy conversion from reqwest's RequestBuilder
    /// into our typed StraicoRequestBuilder while preserving type information.
    ///
    /// # Arguments
    ///
    /// * `value` - The RequestBuilder to convert
    ///
    /// # Returns
    ///
    /// A new StraicoRequestBuilder wrapping the provided RequestBuilder with appropriate type parameters
    fn from(value: RequestBuilder) -> Self {
        StraicoRequestBuilder(value, PhantomData, PhantomData)
    }
}
