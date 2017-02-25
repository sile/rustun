use std::net::SocketAddr;
use slog::{self, Logger};
use futures::{self, Future, BoxFuture};

use {Error, HandleMessage};
use message::{Request, Response, Indication};

#[derive(Debug)]
pub struct DefaultMessageHandler {
    logger: Logger,
}
impl DefaultMessageHandler {
    pub fn new() -> Self {
        DefaultMessageHandler { logger: Logger::root(slog::Discard, o!()) }
    }
    pub fn with_logger(logger: Logger) -> Self {
        DefaultMessageHandler { logger: logger }
    }
}
impl HandleMessage for DefaultMessageHandler {
    type Method = super::Method;
    type Attribute = super::Attribute;
    type HandleCall = BoxFuture<Response<Self::Method, Self::Attribute>, ()>;
    type HandleCast = BoxFuture<(), ()>;
    fn handle_call(&mut self,
                   client: SocketAddr,
                   request: Request<Self::Method, Self::Attribute>)
                   -> Self::HandleCall {
        match *request.method() {
            super::Method::Binding(_) => {
                let mut response = request.into_success_response();
                response.add_attribute(
                    super::Attribute::XorMappedAddress(
                        super::attributes::XorMappedAddress::new(client)));
                futures::finished(Ok(response)).boxed()
            }
        }
    }
    fn handle_cast(&mut self,
                   _client: SocketAddr,
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
