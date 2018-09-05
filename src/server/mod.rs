//! STUN server related components.
use std::net::SocketAddr;
use stun_codec::{Attribute, Method};

use message::{Indication, Request};

pub trait HandleMessage {
    type Method: Method;
    type Attribute: Attribute;

    fn handle_call(
        &self,
        client: Client<Self::Method, Self::Attribute>,
        request: Request<Self::Method, Self::Attribute>,
    ) -> Reply;

    fn handle_cast(
        &self,
        client: Client<Self::Method, Self::Attribute>,
        indication: Indication<Self::Method, Self::Attribute>,
    ) -> NoReply;

    // TODO: handle_error
}

// TODO: name
pub struct Client<M, A>(M, A);
impl<M, A> Client<M, A> {
    pub fn addr(&self) -> SocketAddr {
        panic!()
    }

    pub fn indication_sender(&self) -> IndicationSender<M, A> {
        panic!()
    }
}

pub struct IndicationSender<M, A>(M, A);
impl<M, A> IndicationSender<M, A> {
    pub fn cast(&self, indication: Indication<M, A>) {
        panic!()
    }
}

pub struct Reply;

pub struct NoReply;
