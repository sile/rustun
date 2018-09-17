//! Transport layer abstractions and its built-in implementations.
use fibers_transport::{Result, Transport};
use std::net::SocketAddr;
use stun_codec::{Attribute, DecodedMessage, Message, TransactionId};

pub use self::tcp::StunTcpTransporter;
pub use self::udp::StunUdpTransporter;

pub mod retransmit; // TODO: private
mod tcp;
mod udp;

/// This trait allows the implementation to be used as the transport layer for STUN.
pub trait StunTransport<A>:
    Transport<PeerAddr = SocketAddr, SendItem = Message<A>, RecvItem = DecodedMessage<A>>
where
    A: Attribute,
{
    /// Finishes a request/response transaction.
    fn finish_transaction(&mut self, peer: SocketAddr, transaction_id: TransactionId)
        -> Result<()>;
}
