use std::net::SocketAddr;
use futures::{self, Future, BoxFuture};

use HandleMessage;
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
                   request: Request<Self::Method, Self::Attribute>)
                   -> Self::HandleCall {
        match *request.inner_ref().method() {
            super::Method::Binding(_) => {
                let mut response = request.into_success_response();
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
