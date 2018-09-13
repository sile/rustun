//! Transport layer abstractions and its built-in implementations.
use std::net::SocketAddr;
use stun_codec::{
    Attribute, DecodedMessage, Message, MessageDecoder, MessageEncoder, TransactionId,
};

use Result;

pub use self::retransmit::{RetransmitTransporter, RetransmitTransporterBuilder};
pub use self::tcp::TcpTransporter;
pub use self::udp::{UdpTransporter, UdpTransporterBuilder};

mod retransmit;
mod tcp;
mod udp;

/// A variant of [`UdpTransporter`] that can be used as the transport layer for STUN.
///
/// [`UdpTransporter`]: ./struct.UdpTransporter.html
pub type StunUdpTransporter<A> =
    RetransmitTransporter<A, UdpTransporter<MessageEncoder<A>, MessageDecoder<A>>>;

/// A variant of [`TcpTransporter`] that can be used as the transport layer for STUN.
///
/// [`TcpTransporter`]: ./struct.TcpTransporter.html
pub type StunTcpTransporter<A> = TcpTransporter<MessageEncoder<A>, MessageDecoder<A>>;

/// Transport layer abstraction.
pub trait Transport {
    /// Outgoing message.
    type SendItem;

    /// Incoming message.
    type RecvItem;

    /// Sends the given message to the destination peer.
    fn send(&mut self, peer: SocketAddr, message: Self::SendItem);

    /// Tries to receive a message.
    fn recv(&mut self) -> Option<(SocketAddr, Self::RecvItem)>;

    /// Executes one unit of work needed for sending and receiving messages.
    ///
    /// The return value of `Ok(true)` means that the transporter has been closed.
    fn run_once(&mut self) -> Result<bool>;
}

/// This marker trait indicates that the transport layer of the implementer is unreliable.
///
/// For example, some of sending messages may be discarded.
pub trait UnreliableTransport: Transport {}

/// This trait allows the implementation to be used as the transport layer for STUN.
pub trait StunTransport<A>: Transport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>
where
    A: Attribute,
{
    /// Finishes a request/response transaction.
    fn finish_transaction(&mut self, peer: SocketAddr, transaction_id: TransactionId);
}
