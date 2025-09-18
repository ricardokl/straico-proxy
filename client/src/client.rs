use reqwest::{Client, RequestBuilder};
use serde::Serialize;
use std::{fmt::Display, marker::PhantomData};

use crate::{
    endpoints::{completion::completion_request::CompletionRequest, ApiResponseData},
    error::StraicoError,
};

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
    RequestBuilder,
    PhantomData<Payload>,
    PhantomData<Api>,
);

impl From<Client> for StraicoClient {
    fn from(value: Client) -> Self {
        StraicoClient(value)
    }
}

/// A client for interacting with the Straico API
///
/// Wraps a reqwest::Client and provides convenient methods for making API requests.
/// Can be created using `StraicoClient::new()` or by converting a reqwest::Client
/// using `Into<StraicoClient>`.
#[derive(Clone, Default)]
pub struct StraicoClient(Client);

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

    /// Creates a request builder for the completion endpoint
    ///
    /// # Returns
    ///
    /// A `StraicoRequestBuilder` configured for making completion requests
    pub fn completion<'a>(self) -> StraicoRequestBuilder<NoApiKey, CompletionRequest<'a>> {
        self.0
            .post("https://api.straico.com/v1/prompt/completion")
            .into()
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
    pub fn json<U: Into<T>>(self, payload: U) -> StraicoRequestBuilder<K, PayloadSet> {
        self.0.json(&payload.into()).into()
    }
}

impl StraicoRequestBuilder<ApiKeySet, PayloadSet> {
    /// Sends the configured request to the API and deserializes the JSON response
    ///
    /// This method will send the HTTP request that has been configured with authentication
    /// and payload (if applicable), then attempt to parse the response as JSON into
    /// the expected response type.
    ///
    /// # Returns
    ///
    /// A Future that resolves to a Result containing either:
    /// - The deserialized API response data of type `ApiResponseData<V>`
    /// - A reqwest error if the request fails or JSON parsing fails
    pub async fn send(self) -> Result<ApiResponseData, StraicoError> {
        let response = self.0.send().await?;
        let json = response.json().await?;
        Ok(json)
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
