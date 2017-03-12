//! `HandleMessage` trait implementations.
use std::net::SocketAddr;
use slog::{self, Logger};
use futures::{self, Future, BoxFuture};

use {Error, HandleMessage};
use message::{Request, Response, Indication};
use rfc5389;
use rfc5389::attributes::XorMappedAddress;

/// A `HandleMessage` implementation which only handle `Binding` method.
#[derive(Debug)]
pub struct BindingHandler {
    logger: Logger,
}
impl BindingHandler {
    /// Makes a new `BindingHandler` instance.
    pub fn new() -> Self {
        BindingHandler { logger: Logger::root(slog::Discard, o!()) }
    }

    /// Makes a new `BindingHandler` instance with the speficied logger.
    ///
    /// The logger is used for logging errors that ocurred while handling transactions.
    pub fn with_logger(logger: Logger) -> Self {
        BindingHandler { logger: logger }
    }
}
impl HandleMessage for BindingHandler {
    type Method = rfc5389::methods::Binding;
    type Attribute = rfc5389::Attribute;
    type HandleCall = BoxFuture<Response<Self::Method, Self::Attribute>, ()>;
    type HandleCast = BoxFuture<(), ()>;
    fn handle_call(&mut self,
                   client: SocketAddr,
                   _server: SocketAddr,
                   request: Request<Self::Method, Self::Attribute>)
                   -> Self::HandleCall {
        let mut response = request.into_success_response();
        response.add_attribute(XorMappedAddress::new(client));
        futures::finished(Ok(response)).boxed()
    }
    fn handle_cast(&mut self,
                   _client: SocketAddr,
                   _server: SocketAddr,
                   _message: Indication<Self::Method, Self::Attribute>)
                   -> Self::HandleCast {
        futures::finished(()).boxed()
    }
    fn handle_error(&mut self, client: SocketAddr, error: Error) {
        warn!(self.logger,
              "Cannot handle a message from the client {}: {}",
              client,
              error);
    }
}
