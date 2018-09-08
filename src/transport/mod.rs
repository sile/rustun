//! Transport layer.
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

pub trait Transport {
    type Decoder: Decode;
    type Encoder: Encode;

    fn send(&mut self, peer: SocketAddr, item: <Self::Encoder as Encode>::Item);
    fn recv(&mut self) -> Option<(SocketAddr, <Self::Decoder as Decode>::Item)>;
    fn run_once(&mut self) -> Result<bool>;
}

pub trait UnreliableTransport: Transport {}

pub trait StunTransport<A>:
    Transport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>
where
    A: Attribute,
{
    fn finish_transaction(&mut self, peer: SocketAddr, transaction_id: TransactionId);
}
