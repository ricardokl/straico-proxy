use actix_web::{
    body::MessageBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures::future::LocalBoxFuture;
use log::debug;
use std::future::{ready, Ready};

/// Middleware for logging detailed request information
pub struct RequestDebugger;

impl<S, B> Transform<S, ServiceRequest> for RequestDebugger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestDebuggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestDebuggerMiddleware { service }))
    }
}

pub struct RequestDebuggerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestDebuggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract all needed information before moving req
        let peer_addr = req
            .connection_info()
            .peer_addr()
            .unwrap_or("unknown")
            .to_string();
        let method = req.method().to_string();
        let path = req.path().to_string();
        let version = req.version();

        // Clone headers for logging
        let headers: Vec<_> = req
            .headers()
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();

        debug!("=== Incoming Request ===");
        debug!("From: {}", peer_addr);
        debug!("Method: {} {}", method, path);
        debug!("HTTP Version: {:?}", version);
        debug!("Headers:");

        for (name, value) in headers {
            match value.to_str() {
                Ok(v) => {
                    // Redact Authorization header values for security
                    if name.as_str().eq_ignore_ascii_case("authorization") {
                        debug!("  {}: [REDACTED]", name);
                    } else {
                        debug!("  {}: {}", name, v);
                    }
                }
                Err(_) => {
                    debug!(
                        "  {}: <non-UTF8 value, bytes: {:?}>",
                        name,
                        value.as_bytes()
                    );
                }
            }
        }

        debug!("========================");

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
