//! STUN server related components.
use fibers::sync::mpsc;
use futures::Future;
use std::net::SocketAddr;
use stun_codec::{Attribute, Method};

use message::{Indication, Request, Response};
use {Error, ErrorKind, Result};

pub use self::base::BaseServer;
pub use self::tcp::TcpServer;
pub use self::udp::UdpServer;

pub mod futures {
    //! `Future` trait implementations.
    pub use super::base::BaseServerLoop;
    pub use super::tcp::TcpServerLoop;
    pub use super::udp::UdpServerLoop;
}

mod base;
mod tcp;
mod udp;

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

    /// Handler specific information message type.
    type Info;

    /// Callback method which invoked after the initialization of a server.
    #[allow(unused_variables)]
    fn on_init(&mut self, info_tx: mpsc::Sender<Self::Info>, indication_tx: IndicationSender) {}

    /// Handles the request/response transaction issued by `client`.
    fn handle_call(
        &mut self,
        client: SocketAddr,
        message: Request<Self::Method, Self::Attribute>,
    ) -> Self::HandleCall;

    /// Handles the indication transaction issued by `client`.
    fn handle_cast(
        &mut self,
        client: SocketAddr,
        message: Indication<Self::Method, Self::Attribute>,
    ) -> Self::HandleCast;

    /// Handles the error occurred while processing a transaction issued by `client`.
    fn handle_error(&mut self, client: SocketAddr, error: Error);

    /// Handles the information message.
    #[allow(unused_variables)]
    fn handle_info(&mut self, info: Self::Info) {}
}

/// Indication message sender.
#[derive(Debug, Clone)]
pub struct IndicationSender {
    inner_tx: mpsc::Sender<(SocketAddr, Result<RawMessage>)>,
}
impl IndicationSender {
    fn new(inner_tx: mpsc::Sender<(SocketAddr, Result<RawMessage>)>) -> Self {
        IndicationSender { inner_tx: inner_tx }
    }

    /// Sends the indication message to `peer`.
    pub fn send<M, A>(&self, peer: SocketAddr, indication: Indication<M, A>) -> Result<()>
    where
        M: Method,
        A: Attribute,
    {
        let message = track_try!(RawMessage::try_from_indication(indication));
        track_try!(
            self.inner_tx
                .send((peer, Ok(message)),)
                .map_err(|_| ErrorKind::Other,)
        );
        Ok(())
    }
}
