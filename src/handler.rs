use bytecodec::marker::Never;
use futures::Future;
use std::fmt;
use std::marker::PhantomData;
use std::net::SocketAddr;
use stun_codec::{Attribute, BrokenMessage};

use message::{Indication, Request, Response};

pub enum Action<T> {
    Reply(T),
    FutureReply(Box<Future<Item = T, Error = Never> + Send + 'static>),
    NoReply,
    FutureNoReply(Box<Future<Item = (), Error = Never> + Send + 'static>),
}
impl<T: fmt::Debug> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::Reply(t) => write!(f, "Reply({:?})", t),
            Action::FutureReply(_) => write!(f, "FutureReply(_)"),
            Action::NoReply => write!(f, "NoReply"),
            Action::FutureNoReply(_) => write!(f, "FutureNoReply(_)"),
        }
    }
}

pub trait HandleMessage {
    type Attribute: Attribute;

    fn handle_call(
        &mut self,
        peer: SocketAddr,
        request: Request<Self::Attribute>,
    ) -> Action<Response<Self::Attribute>>;

    fn handle_cast(
        &mut self,
        peer: SocketAddr,
        indication: Indication<Self::Attribute>,
    ) -> Action<Never>;

    fn handle_broken_message(
        &mut self,
        peer: SocketAddr,
        message: BrokenMessage,
    ) -> Action<Response<Self::Attribute>>;
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
impl<A: Attribute> Default for NoopMessageHandler<A> {
    fn default() -> Self {
        Self::new()
    }
}
impl<A: Attribute> HandleMessage for NoopMessageHandler<A> {
    type Attribute = A;

    fn handle_call(
        &mut self,
        _peer: SocketAddr,
        _request: Request<Self::Attribute>,
    ) -> Action<Response<Self::Attribute>> {
        Action::NoReply
    }

    fn handle_cast(
        &mut self,
        _peer: SocketAddr,
        _indication: Indication<Self::Attribute>,
    ) -> Action<Never> {
        Action::NoReply
    }

    fn handle_broken_message(
        &mut self,
        _peer: SocketAddr,
        _message: BrokenMessage,
    ) -> Action<Response<Self::Attribute>> {
        Action::NoReply
    }
}
