use std::net::SocketAddr;
use futures::{self, Future, BoxFuture};

use {HandleMessage, Method};
use message::{Request, Response, Indication};

#[derive(Debug)]
pub struct DefaultMessageHandler;
impl HandleMessage for DefaultMessageHandler {
    type Method = super::Method;
    type Attribute = super::Attribute;
    type HandleCall = BoxFuture<Response<Self::Method, Self::Attribute>, ()>;
    type HandleCast = BoxFuture<(), ()>;
    fn handle_call(&mut self,
                   client: SocketAddr,
                   message: Request<Self::Method, Self::Attribute>)
                   -> Self::HandleCall {
        let message = message.into_inner();
        match *message.method() {
            super::Method::Binding(_) => {
                let mut response = super::Method::binding().success_response();
                response.inner_mut().add_attribute(
                    super::Attribute::XorMappedAddress(
                        super::attributes::XorMappedAddress::new(client)));
                futures::finished(response).boxed()
            }
        }
    }
    fn handle_cast(&mut self,
                   _client: SocketAddr,
                   _message: Indication<Self::Method, Self::Attribute>)
                   -> Self::HandleCast {
        futures::finished(()).boxed()
    }
}
