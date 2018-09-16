use fibers_transport::{PollRecv, PollSend, Result, Transport, UdpTransport};
use std::net::SocketAddr;
use stun_codec::{Attribute, DecodedMessage, Message, TransactionId};

use super::retransmit::RetransmitTransporter;
use super::StunTransport;

// TODO: bulider

#[derive(Debug)]
pub struct StunUdpTransporter<A, T> {
    inner: RetransmitTransporter<A, T>,
}
impl<A, T> StunUdpTransporter<A, T>
where
    A: Attribute,
    T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    pub fn new(inner: T) -> Self {
        StunUdpTransporter {
            inner: RetransmitTransporter::new(inner),
        }
    }
}
impl<A, T> Transport for StunUdpTransporter<A, T>
where
    A: Attribute,
    T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    type PeerAddr = SocketAddr;
    type SendItem = Message<A>;
    type RecvItem = DecodedMessage<A>;

    fn start_send(&mut self, peer: Self::PeerAddr, item: Self::SendItem) -> Result<()> {
        track!(self.inner.start_send(peer, item))
    }

    fn poll_send(&mut self) -> PollSend {
        track!(self.inner.poll_send())
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)> {
        track!(self.inner.poll_recv())
    }
}
impl<A, T> StunTransport<A> for StunUdpTransporter<A, T>
where
    A: Attribute,
    T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    fn finish_transaction(&mut self, peer: SocketAddr, transaction_id: TransactionId) {
        self.inner.finish_transaction(peer, transaction_id);
    }
}
