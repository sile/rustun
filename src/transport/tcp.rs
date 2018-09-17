use fibers_transport::{ErrorKind, PollRecv, PollSend, Result, TcpTransport, Transport};
use futures::Async;
use std::net::SocketAddr;
use stun_codec::{Attribute, DecodedMessage, Message, TransactionId};

use super::StunTransport;

/// TCP transport layer that can be used for STUN.
#[derive(Debug)]
pub struct StunTcpTransporter<T> {
    inner: T,
}
impl<A, T> StunTcpTransporter<T>
where
    A: Attribute,
    T: TcpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    /// Makes a new `StunTcpTransporter` instance.
    pub fn new(inner: T) -> Self {
        StunTcpTransporter { inner }
    }

    /// Returns a reference to the inner transporter.
    pub fn inner_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the inner transporter.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
impl<A, T> Transport for StunTcpTransporter<T>
where
    A: Attribute,
    T: TcpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    type PeerAddr = SocketAddr;
    type SendItem = Message<A>;
    type RecvItem = DecodedMessage<A>;

    fn start_send(&mut self, peer: Self::PeerAddr, item: Self::SendItem) -> Result<()> {
        track_assert_eq!(
            peer,
            self.inner.peer_addr(),
            ErrorKind::InvalidInput,
            "Unexpected destination peer"
        );
        track!(self.inner.start_send((), item))
    }

    fn poll_send(&mut self) -> PollSend {
        track!(self.inner.poll_send())
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)> {
        match track!(self.inner.poll_recv())? {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(None) => Ok(Async::Ready(None)),
            Async::Ready(Some((_, item))) => Ok(Async::Ready(Some((self.inner.peer_addr(), item)))),
        }
    }
}
impl<A, T> StunTransport<A> for StunTcpTransporter<T>
where
    A: Attribute,
    T: TcpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    fn finish_transaction(
        &mut self,
        _peer: SocketAddr,
        _transaction_id: TransactionId,
    ) -> Result<()> {
        Ok(())
    }
}
