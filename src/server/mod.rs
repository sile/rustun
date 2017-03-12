//! STUN server related components.
use std::net::SocketAddr;
use futures::Future;

use {Method, Attribute, Error};
use message::{Indication, Request, Response};

pub use self::base::BaseServer;
pub use self::udp::UdpServer;
pub use self::tcp::TcpServer;

pub mod futures {
    //! `Future` trait implementations.
    pub use super::base::BaseServerLoop;
    pub use super::udp::UdpServerLoop;
    pub use super::tcp::TcpServerLoop;
}

mod base;
mod udp;
mod tcp;

/// This trait allows to handle transactions issued by clients.
pub trait HandleMessage {
    /// STUN method type that this implementation can handle.
    type Method: Method;

    /// STUN attribute type that this implementation can handle.
    type Attribute: Attribute;

    /// `Future` type for handling request/response transactions.
    type HandleCall: Future<Item = Response<Self::Method, Self::Attribute>, Error = ()>;

    /// `Future` type for handling indication transactions.
    type HandleCast: Future<Item = (), Error = ()>;

    /// Handles the request/response transaction issued by `client`.
    fn handle_call(&mut self,
                   client: SocketAddr,
                   message: Request<Self::Method, Self::Attribute>)
                   -> Self::HandleCall;

    /// Handles the indication transaction issued by `client`.
    fn handle_cast(&mut self,
                   client: SocketAddr,
                   message: Indication<Self::Method, Self::Attribute>)
                   -> Self::HandleCast;

    /// Handles the error occurred while processing a transaction issued by `client`.
    fn handle_error(&mut self, client: SocketAddr, error: Error);
}
