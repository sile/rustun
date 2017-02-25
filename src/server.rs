use std::net::SocketAddr;
use futures::Future;

use {Method, Attribute, Error};
use message::{Indication, Request, Response};

pub trait HandleMessage {
    type Method: Method;
    type Attribute: Attribute;
    type HandleCall: Future<Item = Response<Self::Method, Self::Attribute>, Error = ()> + Send + 'static;
    type HandleCast: Future<Item = (), Error = ()> + Send + 'static;
    fn handle_call(&mut self,
                   client: SocketAddr,
                   message: Request<Self::Method, Self::Attribute>)
                   -> Self::HandleCall;
    fn handle_cast(&mut self,
                   client: SocketAddr,
                   message: Indication<Self::Method, Self::Attribute>)
                   -> Self::HandleCast;
    fn handle_error(&mut self, client: SocketAddr, error: Error);
}
