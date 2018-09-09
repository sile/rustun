use futures::{Future, Poll};
use std::marker::PhantomData;
use std::net::SocketAddr;
use stun_codec::Attribute;

use handler::HandleMessage;
use message::{Indication, Request, Response};
use transport::StunTransport;
use Error;

#[derive(Debug)]
pub struct Agent<A, T, H>
where
    A: Attribute,
    T: StunTransport<A>,
    H: HandleMessage<Attribute = A>,
{
    transporter: T,
    message_handler: H,
    _phantom: PhantomData<A>,
}
impl<A, T, H> Agent<A, T, H>
where
    A: Attribute,
    T: StunTransport<A>,
    H: HandleMessage<Attribute = A>,
{
    pub fn new(transporter: T, message_handler: H) -> Self {
        Agent {
            transporter,
            message_handler,
            _phantom: PhantomData,
        }
    }

    pub fn call(
        &mut self,
        _peer: SocketAddr,
        _request: Request<A>,
    ) -> impl Future<Item = Response<A>, Error = Error> {
        ::futures::finished(panic!())
    }

    pub fn cast(&mut self, _peer: SocketAddr, _indication: Indication<A>) {
        panic!()
    }

    pub fn message_handler_ref(&self) -> &H {
        &self.message_handler
    }

    pub fn message_handler_mut(&mut self) -> &mut H {
        &mut self.message_handler
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
