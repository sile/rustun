use bytecodec::marker::Never;
use futures::{Future, Poll, Stream};
use std::marker::PhantomData;
use std::net::SocketAddr;
use stun_codec::Attribute;

use message::{Indication, Request, Response};
use transport::{AnyMethod, StunTransport};
use {AsyncResult, Error};

#[derive(Debug)]
pub struct Agent<A, T, H> {
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

    pub fn call<M>(
        &mut self,
        _peer: SocketAddr,
        _request: Request<M, A>,
    ) -> AsyncResult<Response<M, A>> {
        panic!()
    }

    pub fn cast<M>(&mut self, _peer: SocketAddr, _indication: Indication<M, A>) {
        panic!()
    }

    pub fn message_handler_ref(&self) -> &H {
        &self.handler
    }

    pub fn message_handler_mut(&mut self) -> &mut H {
        &mut self.handler
    }
}
impl<A, T, H> Stream for Agent<A, T, H>
where
    A: Attribute,
    T: StunTransport<A>,
    H: HandleMessage<Attribute = A>,
{
    type Item = H::Event;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        panic!()
    }
}

// TODO: action?
pub enum Reply<T> {
    Immediate(T),
    Future(Box<Future<Item = T, Error = Never> + Send + 'static>),
    NoReply, // future or immediate
}

pub trait HandleMessage {
    type Attribute: Attribute;
    type Event;

    fn handle_call(
        &mut self,
        peer: SocketAddr,
        request: Request<AnyMethod, Self::Attribute>,
    ) -> Reply<Response<AnyMethod, Self::Attribute>>;

    fn handle_cast(
        &mut self,
        peer: SocketAddr,
        indication: Indication<AnyMethod, Self::Attribute>,
    ) -> Reply<Never>;

    fn handle_error(
        &mut self,
        peer: SocketAddr,
        error: Error,
    ) -> Reply<Response<AnyMethod, Self::Attribute>>;
}

#[derive(Debug)]
pub struct NoopMessageHandler<A> {
    _phantom: PhantomData<A>,
}
impl<A: Attribute> NoopMessageHandler<A> {
    pub fn new() -> Self {
        NoopMessageHandler {
            _phantom: PhantomData,
        }
    }
}
impl<A: Attribute> HandleMessage for NoopMessageHandler<A> {
    type Attribute = A;
    type Event = Never;

    // fn init(&mut self, agent: AgentHandle);

    fn handle_call(
        &mut self,
        _peer: SocketAddr,
        _request: Request<AnyMethod, Self::Attribute>,
    ) -> Reply<Response<AnyMethod, Self::Attribute>> {
        Reply::NoReply
    }

    fn handle_cast(
        &mut self,
        _peer: SocketAddr,
        _indication: Indication<AnyMethod, Self::Attribute>,
    ) -> Reply<Never> {
        Reply::NoReply
    }

    fn handle_error(
        &mut self,
        _peer: SocketAddr,
        _error: Error,
    ) -> Reply<Response<AnyMethod, Self::Attribute>> {
        Reply::NoReply
    }
}
