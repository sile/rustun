use futures::{Future, Poll};
use std::marker::PhantomData;
use std::net::SocketAddr;
use stun_codec::Attribute;

use handler::HandleMessage;
use message::{Indication, Request, Response};
use transport::StunTransport;
use {AsyncResult, Error};

#[derive(Debug)]
pub struct Agent<A, T, H>
where
    A: Attribute,
    T: StunTransport<A>,
    H: HandleMessage<Attribute = A>,
{
    transporter: T,
    handler: H,
    _phantom: PhantomData<A>,
}
impl<A, T, H> Agent<A, T, H>
where
    A: Attribute,
    T: StunTransport<A>,
    H: HandleMessage<Attribute = A>,
{
    pub fn new(transporter: T, handler: H) -> Self {
        Agent {
            transporter,
            handler,
            _phantom: PhantomData,
        }
    }

    pub fn call(&mut self, _peer: SocketAddr, _request: Request<A>) -> AsyncResult<Response<A>> {
        panic!()
    }

    pub fn cast(&mut self, _peer: SocketAddr, _indication: Indication<A>) {
        panic!()
    }

    pub fn message_handler_ref(&self) -> &H {
        &self.handler
    }

    pub fn message_handler_mut(&mut self) -> &mut H {
        &mut self.handler
    }
}
impl<A, T, H> Future for Agent<A, T, H>
where
    A: Attribute,
    T: StunTransport<A>,
    H: HandleMessage<Attribute = A>,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        panic!()
    }
}
impl<A, T, H> Drop for Agent<A, T, H>
where
    A: Attribute,
    T: StunTransport<A>,
    H: HandleMessage<Attribute = A>,
{
    fn drop(&mut self) {
        let _ = self.transporter.run_once();
    }
}
