use std::net::SocketAddr;
use futures::Future;

use {Method, Attribute};
use message::{Indication, Request, Response};

pub trait HandleMessage<M, A>
    where M: Method,
          A: Attribute
{
    type HandleCall: Future<Item = Response<M, A>, Error = ()>;
    type HandleCast: Future<Item = (), Error = ()>;
    fn handle_call(&mut self, client: SocketAddr, message: Request<M, A>) -> Self::HandleCall;
    fn handle_cast(&mut self, client: SocketAddr, message: Indication<M, A>) -> Self::HandleCast;
}
