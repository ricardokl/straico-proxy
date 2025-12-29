use reqwest::{Client, ClientBuilder, RequestBuilder};
use std::{fmt::Display, future::Future, marker::PhantomData, time::Duration};

use crate::endpoints::Endpoint;

const BASE_URL: &str = "https://api.straico.com";

/// Represents the state where no API key has been set for the request
pub struct NoApiKey;
/// Represents the state where an API key has been set for the request
pub struct ApiKeySet;

/// Builder for making requests to Straico API endpoints
///
/// # Type Parameters
///
/// * `Api` - Represents the authentication state (NoApiKey or ApiKeySet)
/// * `Payload` - Represents the request payload state
/// * `Response` - The expected response type from the API
pub struct StraicoRequestBuilder<Api, Payload>(
    pub RequestBuilder,
    pub PhantomData<Payload>,
    pub PhantomData<Api>,
);

impl From<Client> for StraicoClient {
    fn from(value: Client) -> Self {
        Self { client: value }
    }
}

/// A client for interacting with the Straico API
///
/// Wraps a reqwest::Client and provides convenient methods for making API requests.
/// Can be created using `StraicoClient::new()` or by converting a reqwest::Client
/// using `Into<StraicoClient>`.
#[derive(Clone)]
pub struct StraicoClient {
    pub client: reqwest::Client,
}

pub struct StraicoClientBuilder {
    pub client: ClientBuilder,
}

impl Default for StraicoClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
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

    /// Creates a request builder for the given endpoint.
    pub fn request<E: Endpoint>(
        self,
        endpoint: &E,
    ) -> StraicoRequestBuilder<NoApiKey, E::Request> {
        let url = format!("{}{}", BASE_URL, endpoint.path());
        self.client
            .request(endpoint.method(), &url)
            .json(endpoint.request_body())
            .into()
    }

    pub fn builder() -> StraicoClientBuilder {
        StraicoClientBuilder {
            client: reqwest::Client::builder(),
        }
    }
}

impl StraicoClientBuilder {
    pub fn pool_max_idle_per_host(self, max: usize) -> StraicoClientBuilder {
        Self {
            client: self.client.pool_max_idle_per_host(max),
        }
    }

    pub fn pool_idle_timeout<D: Into<Option<Duration>>>(self, val: D) -> StraicoClientBuilder {
        Self {
            client: self.client.pool_idle_timeout(val),
        }
    }

    pub fn tcp_keepalive<D: Into<Option<Duration>>>(self, val: D) -> StraicoClientBuilder {
        Self {
            client: self.client.tcp_keepalive(val),
        }
    }

    pub fn timeout(self, timeout: Duration) -> StraicoClientBuilder {
        Self {
            client: self.client.timeout(timeout),
        }
    }

    pub fn build(self) -> Result<StraicoClient, reqwest::Error> {
        Ok(StraicoClient {
            client: self.client.build()?,
        })
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
