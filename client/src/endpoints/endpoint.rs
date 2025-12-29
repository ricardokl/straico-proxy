use reqwest::Method;
use serde::{de::DeserializeOwned, Serialize};

/// A trait representing a Straico API endpoint.
///
/// This trait defines the necessary components for an API endpoint, including the request
/// and response types, the HTTP method, and the URL path. By implementing this trait,
/// a type can be used with the generic `request` method of the `StraicoClient`.
pub trait Endpoint {
    /// The type of the request body. Must be serializable.
    type Request: Serialize;
    /// The type of the response body. Must be deserializable.
    type Response: DeserializeOwned;

    /// Returns the HTTP method for this endpoint.
    fn method(&self) -> Method;

    /// Returns the URL path for this endpoint.
    fn path(&self) -> &str;

    /// Returns the request body.
    fn request_body(&self) -> &Self::Request;
}
