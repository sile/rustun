//! Transport layer abstractions and its built-in implementations.
use bytecodec::{Decode, Encode};
use std::net::SocketAddr;
use stun_codec::{Attribute, MessageDecoder, MessageEncoder, TransactionId};

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
    RetransmitTransporter<A, UdpTransporter<MessageDecoder<A>, MessageEncoder<A>>>;

/// Transport layer abstraction.
pub trait Transport {
    /// The decoder used for decoding incoming messages.
    type Decoder: Decode;

    /// The encoder used for encoding outgoing messages.
    type Encoder: Encode;

    /// Sends the given message to the destination peer.
    fn send(&mut self, peer: SocketAddr, message: <Self::Encoder as Encode>::Item);

    /// Tries to receive a message.
    fn recv(&mut self) -> Option<(SocketAddr, <Self::Decoder as Decode>::Item)>;

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
pub trait StunTransport<A>:
    Transport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>
where
    A: Attribute,
{
    /// Finishes a request/response transaction.
    fn finish_transaction(&mut self, peer: SocketAddr, transaction_id: TransactionId);
}
