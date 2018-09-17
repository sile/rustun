//! Transport layer abstractions and its built-in implementations.
use fibers_transport::{FixedPeerTransporter, PeerAddr, Result, Transport};
use stun_codec::{Attribute, DecodedMessage, Message, TransactionId};

pub use self::tcp::StunTcpTransporter;
pub use self::udp::{StunUdpTransporter, StunUdpTransporterBuilder};

mod tcp;
mod udp;

/// This trait allows the implementation to be used as the transport layer for STUN.
pub trait StunTransport<A>: Transport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>
where
    A: Attribute,
{
    /// Finishes a request/response transaction.
    fn finish_transaction(
        &mut self,
        peer: &Self::PeerAddr,
        transaction_id: TransactionId,
    ) -> Result<()>;
}
impl<A, T, P> StunTransport<A> for FixedPeerTransporter<T, P>
where
    A: Attribute,
    T: StunTransport<A>,
    P: PeerAddr,
{
    fn finish_transaction(&mut self, _peer: &P, transaction_id: TransactionId) -> Result<()> {
        let peer = self.interior_peer().clone();
        track!(self.inner_mut().finish_transaction(&peer, transaction_id))
    }
}
