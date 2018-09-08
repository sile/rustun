//! Transport layer.
use bytecodec::{Decode, Encode};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::time::Duration;
use stun_codec::{Attribute, MessageDecoder, MessageEncoder, TransactionId};

use constants;
use Result;

pub use self::tcp::TcpTransporter;
pub use self::udp::{UdpTransporter, UdpTransporterBuilder};

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
    fn finish_transaction(&mut self, transaction_id: TransactionId);
}

#[derive(Debug, Clone)]
pub struct RetransmitTransporterBuilder {
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}
impl RetransmitTransporterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finish<A, T>(&self, inner: T) -> RetransmitTransporter<A, T>
    where
        A: Attribute,
        T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
    {
        RetransmitTransporter {
            inner,
            _phantom: PhantomData,
            current_rto: self.rto,
            cached_rto: self.rto,
            rto: self.rto,
            rto_cache_duration: self.rto_cache_duration,
            min_transaction_interval: self.min_transaction_interval,
            max_outstanding_transactions: self.max_outstanding_transactions,
        }
    }
}
impl Default for RetransmitTransporterBuilder {
    fn default() -> Self {
        RetransmitTransporterBuilder {
            rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(constants::DEFAULT_RTO_CACHE_DURATION_MS),
            min_transaction_interval: Duration::from_millis(
                constants::DEFAULT_MIN_TRANSACTION_INTERVAL_MS,
            ),
            max_outstanding_transactions: constants::DEFAULT_MAX_OUTSTANDING_TRANSACTIONS,
        }
    }
}

#[derive(Debug)]
pub struct RetransmitTransporter<A, T> {
    inner: T,
    _phantom: PhantomData<A>,
    current_rto: Duration,
    cached_rto: Duration,
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}
impl<A, T> RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
{
    pub fn new(inner: T) -> Self {
        RetransmitTransporterBuilder::new().finish(inner)
    }

    pub fn inner_ref(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
impl<A, T> Transport for RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
{
    type Decoder = MessageDecoder<A>;
    type Encoder = MessageEncoder<A>;

    // TODO:
    fn send(&mut self, peer: SocketAddr, item: <Self::Encoder as Encode>::Item) {
        self.inner.send(peer, item);
    }
    fn recv(&mut self) -> Option<(SocketAddr, <Self::Decoder as Decode>::Item)> {
        self.inner.recv()
    }
    fn run_once(&mut self) -> Result<bool> {
        track!(self.inner.run_once())
    }
}
impl<A, T> StunTransport<A> for RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
{
    fn finish_transaction(&mut self, _transaction_id: TransactionId) {
        panic!("TODO")
    }
}
