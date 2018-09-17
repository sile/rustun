use fibers_timeout_queue::TimeoutQueue;
use fibers_transport::{PollRecv, PollSend, Result, Transport, UdpTransport};
use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use stun_codec::{Attribute, DecodedMessage, Message, MessageClass, TransactionId};

use super::StunTransport;

/// [`StunUdpTransporter`] builder.
///
/// [`StunUdpTransporter`]: ./struct.StunUdpTransporter.html
#[derive(Debug, Clone)]
pub struct StunUdpTransporterBuilder {
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}
impl StunUdpTransporterBuilder {
    /// The default value of RTO (Retransmission TimeOut).
    ///
    /// > A client SHOULD retransmit a STUN request message starting with an
    /// > interval of RTO ("Retransmission TimeOut"), doubling after each
    /// > retransmission.  The RTO is an estimate of the round-trip time (RTT),
    /// > and is computed as described in RFC 2988 [RFC2988], with two
    /// > exceptions.  First, the initial value for RTO SHOULD be configurable
    /// > (rather than the 3 s recommended in RFC 2988) and SHOULD be greater
    /// > than **500 ms**.
    /// >
    /// > [RFC 5389 -- 7.2.1. Sending over UDP]
    ///
    /// [RFC 5389 -- 7.2.1. Sending over UDP]: https://tools.ietf.org/html/rfc5389#section-7.2.1
    pub const DEFAULT_RTO_MS: u64 = 500;

    /// The default duration preserving a cached RTO (Retransmission TimeOut).
    ///
    /// > The value for RTO SHOULD be cached by a client after the completion
    /// > of the transaction, and used as the starting value for RTO for the
    /// > next transaction to the same server (based on equality of IP
    /// > address).  The value SHOULD be considered stale and discarded after
    /// > **10 minutes**.
    /// >
    /// > [RFC 5389 -- 7.2.1. Sending over UDP]
    ///
    /// [RFC 5389 -- 7.2.1. Sending over UDP]: https://tools.ietf.org/html/rfc5389#section-7.2.1
    pub const DEFAULT_RTO_CACHE_DURATION_MS: u64 = 10 * 60 * 1000;

    /// The default max concurrent transactions by a client to a server.
    ///
    /// > At any time, a client MAY have multiple outstanding STUN requests
    /// > with the same STUN server (that is, multiple transactions in
    /// > progress, with different transaction IDs).  Absent other limits to
    /// > the rate of new transactions (such as those specified by ICE for
    /// > connectivity checks or when STUN is run over TCP), a client SHOULD
    /// > space new transactions to a server by RTO and SHOULD limit itself to
    /// > **ten outstanding transactions** to the same server.
    /// >
    /// > [RFC 5389 -- 7.2. Sending the Request or Indication]
    ///
    /// [RFC 5389 -- 7.2. Sending the Request or Indication]: https://tools.ietf.org/html/rfc5389#section-7.2
    pub const DEFAULT_MAX_OUTSTANDING_TRANSACTIONS: usize = 10;

    /// The default interval between transactions issued by a client to a serve.
    ///
    /// > At any time, a client MAY have multiple outstanding STUN requests
    /// > with the same STUN server (that is, multiple transactions in
    /// > progress, with different transaction IDs).  Absent other limits to
    /// > the rate of new transactions (such as those specified by ICE for
    /// > connectivity checks or when STUN is run over TCP), **a client SHOULD
    /// > space new transactions to a server by RTO** and SHOULD limit itself to
    /// > ten outstanding transactions to the same server.
    /// >
    /// > [RFC 5389 -- 7.2. Sending the Request or Indication]
    ///
    /// [RFC 5389 -- 7.2. Sending the Request or Indication]: https://tools.ietf.org/html/rfc5389#section-7.2
    pub const DEFAULT_MIN_TRANSACTION_INTERVAL_MS: u64 = Self::DEFAULT_RTO_MS;

    /// Makes a new `StunUdpTransporterBuilder` instance with the default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the RTO of the resulting instance.
    ///
    /// The default value is `Duration::from_millis(DEFAULT_RTO_MS)`.
    pub fn rto(&mut self, rto: Duration) -> &mut Self {
        self.rto = rto;
        self
    }

    /// Sets the RTO cache duration of the resulting instance.
    ///
    /// The default value is `Duration::from_millis(DEFAULT_RTO_CACHE_DURATION_MS)`.
    pub fn rto_cache_duration(&mut self, duration: Duration) -> &mut Self {
        self.rto_cache_duration = duration;
        self
    }

    /// Sets the minimum interval of the consecutive request/response transactions of
    /// the resulting instance.
    ///
    /// The default value is `Duration::from_millis(DEFAULT_MIN_TRANSACTION_INTERVAL_MS)`.
    pub fn min_transaction_interval(&mut self, interval: Duration) -> &mut Self {
        self.min_transaction_interval = interval;
        self
    }

    /// Sets the number of the maximum outstanding transactions of the resulting instance.
    ///
    /// The default value is `DEFAULT_MAX_OUTSTANDING_TRANSACTIONS`.
    pub fn max_outstanding_transactions(&mut self, max: usize) -> &mut Self {
        self.max_outstanding_transactions = max;
        self
    }

    /// Makes a new `StunUdpTransporter` instance with the given settings.
    pub fn finish<A, T>(&self, inner: T) -> StunUdpTransporter<A, T>
    where
        A: Attribute,
        T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
    {
        let inner = RetransmitTransporter {
            inner,
            _phantom: PhantomData,
            timeout_queue: TimeoutQueue::new(),
            peers: HashMap::new(),
            rto: self.rto,
            rto_cache_duration: self.rto_cache_duration,
            min_transaction_interval: self.min_transaction_interval,
            max_outstanding_transactions: self.max_outstanding_transactions,
        };
        StunUdpTransporter { inner }
    }
}
impl Default for StunUdpTransporterBuilder {
    fn default() -> Self {
        StunUdpTransporterBuilder {
            rto: Duration::from_millis(Self::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(Self::DEFAULT_RTO_CACHE_DURATION_MS),
            min_transaction_interval: Duration::from_millis(
                Self::DEFAULT_MIN_TRANSACTION_INTERVAL_MS,
            ),
            max_outstanding_transactions: Self::DEFAULT_MAX_OUTSTANDING_TRANSACTIONS,
        }
    }
}

/// UDP transport layer that can be used for STUN.
#[derive(Debug)]
pub struct StunUdpTransporter<A, T> {
    inner: RetransmitTransporter<A, T>,
}
impl<A, T> StunUdpTransporter<A, T>
where
    A: Attribute,
    T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    /// Makes a new `StunUdpTransporter` instance.
    ///
    /// This is equivalent to `StunUdpTransporterBuilder::new().finish(inner)`.
    pub fn new(inner: T) -> Self {
        StunUdpTransporterBuilder::new().finish(inner)
    }

    /// Returns a reference to the inner transporter.
    pub fn inner_ref(&self) -> &T {
        &self.inner.inner
    }

    /// Returns a mutable reference to the inner transporter.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner.inner
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
    fn finish_transaction(
        &mut self,
        peer: SocketAddr,
        transaction_id: TransactionId,
    ) -> Result<()> {
        track!(self.inner.finish_transaction(peer, transaction_id))
    }
}

/// An implementation of [`StunTransport`] that retransmits request messages for improving reliability.
///
/// [`StunTransport`]: ./trait.StunTransport.html
#[derive(Debug)]
struct RetransmitTransporter<A, T> {
    inner: T,
    _phantom: PhantomData<A>,
    timeout_queue: TimeoutQueue<TimeoutEntry<A>>,
    peers: HashMap<SocketAddr, PeerState<A>>,
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}
impl<A, T> RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    fn waiting_time(&self, peer: SocketAddr) -> Option<Duration> {
        self.peers[&peer]
            .last_transaction_start_time
            .elapsed()
            .ok()
            .and_then(|d| self.min_transaction_interval.checked_sub(d))
    }

    fn peer_mut(&mut self, peer: SocketAddr) -> &mut PeerState<A> {
        self.peers.get_mut(&peer).expect("never fails")
    }

    #[cfg_attr(feature = "cargo-clippy", allow(map_entry))]
    fn start_transaction(
        &mut self,
        peer: SocketAddr,
        request: Message<A>,
        first: bool,
    ) -> Result<()> {
        if !self.peers.contains_key(&peer) {
            self.peers.insert(peer, PeerState::new(peer, self.rto));
        }

        if self.peers[&peer].waiting {
            self.peer_mut(peer).pending(request, first);
        } else if let Some(duration) = self.waiting_time(peer) {
            self.peer_mut(peer).waiting = true;
            self.timeout_queue
                .push(TimeoutEntry::AllowNextRequest { peer }, duration);
            self.peer_mut(peer).pending(request, first);
        } else if self.peers[&peer].transactions.len() >= self.max_outstanding_transactions {
            self.peer_mut(peer).pending(request, first);
        } else {
            track!(self.inner.start_send(peer, request.clone()))?;
            let timeout = self.peer_mut(peer).start_transaction(request);
            self.timeout_queue.push(timeout.0, timeout.1);
        }
        Ok(())
    }

    fn poll_timeout(&mut self) -> Option<TimeoutEntry<A>> {
        let peers = &self.peers;
        self.timeout_queue.filter_pop(|entry| {
            if let TimeoutEntry::Retransmit { peer, request, .. } = entry {
                peers.get(&peer).map_or(false, |p| {
                    p.transactions.contains(&request.transaction_id())
                })
            } else {
                true
            }
        })
    }

    fn handle_pending_request(&mut self, peer: SocketAddr) -> Result<()> {
        if !self.peers.contains_key(&peer) {
            return Ok(());
        }
        if let Some(request) = self.peer_mut(peer).pop_pending_request() {
            track!(self.start_transaction(peer, request, false))?;
        }
        if self.peers[&peer].is_idle() {
            self.peers.remove(&peer);
        }
        Ok(())
    }

    fn handle_retransmit(
        &mut self,
        peer: SocketAddr,
        request: Message<A>,
        rto: Duration,
    ) -> Result<()> {
        if let Some(p) = self.peers.get_mut(&peer) {
            if let Some(request) = p.retransmit(
                request,
                rto,
                self.rto_cache_duration,
                &mut self.timeout_queue,
            ) {
                track!(self.inner.start_send(peer, request))?;
            }
        }
        Ok(())
    }
}
impl<A, T> Transport for RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    type PeerAddr = SocketAddr;
    type SendItem = Message<A>;
    type RecvItem = DecodedMessage<A>;

    fn start_send(&mut self, peer: SocketAddr, item: Self::SendItem) -> Result<()> {
        if item.class() == MessageClass::Request {
            track!(self.start_transaction(peer, item, true))
        } else {
            track!(self.inner.start_send(peer, item))
        }
    }

    fn poll_send(&mut self) -> PollSend {
        while let Some(entry) = self.poll_timeout() {
            match entry {
                TimeoutEntry::Retransmit {
                    peer,
                    request,
                    next_rto,
                } => {
                    track!(self.handle_retransmit(peer, request, next_rto))?;
                }
                TimeoutEntry::ExpireRtoCache { peer, cached_rto } => {
                    if let Some(p) = self.peers.get_mut(&peer) {
                        if p.cached_rto == cached_rto {
                            p.cached_rto = self.rto;
                        }
                    }
                }
                TimeoutEntry::AllowNextRequest { peer } => {
                    self.peer_mut(peer).waiting = false;
                    track!(self.handle_pending_request(peer))?;
                }
            }
        }

        track!(self.inner.poll_send())
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)> {
        track!(self.inner.poll_recv())
    }
}
impl<A, T> StunTransport<A> for RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UdpTransport<SendItem = Message<A>, RecvItem = DecodedMessage<A>>,
{
    fn finish_transaction(
        &mut self,
        peer: SocketAddr,
        transaction_id: TransactionId,
    ) -> Result<()> {
        if let Some(p) = self.peers.get_mut(&peer) {
            p.finish_transaction(transaction_id);
        }
        track!(self.handle_pending_request(peer))
    }
}

#[derive(Debug)]
enum TimeoutEntry<A> {
    Retransmit {
        peer: SocketAddr,
        request: Message<A>,
        next_rto: Duration,
    },
    ExpireRtoCache {
        peer: SocketAddr,
        cached_rto: Duration,
    },
    AllowNextRequest {
        peer: SocketAddr,
    },
}

#[derive(Debug)]
struct PeerState<A> {
    peer: SocketAddr,
    transactions: HashSet<TransactionId>,
    pending_requests: VecDeque<Message<A>>,
    waiting: bool,
    last_transaction_start_time: SystemTime,
    current_rto: Duration,
    cached_rto: Duration,
}
impl<A: Attribute> PeerState<A> {
    fn new(peer: SocketAddr, rto: Duration) -> Self {
        PeerState {
            peer,
            transactions: HashSet::new(),
            pending_requests: VecDeque::new(),
            waiting: false,
            last_transaction_start_time: UNIX_EPOCH,
            current_rto: rto,
            cached_rto: rto,
        }
    }

    fn pending(&mut self, request: Message<A>, first: bool) {
        if first {
            self.pending_requests.push_back(request);
        } else {
            self.pending_requests.push_front(request);
        }
    }

    fn is_idle(&self) -> bool {
        self.transactions.is_empty() && !self.waiting
    }

    fn pop_pending_request(&mut self) -> Option<Message<A>> {
        while let Some(request) = self.pending_requests.pop_front() {
            if self.transactions.contains(&request.transaction_id()) {
                return Some(request);
            }
        }
        None
    }

    fn retransmit(
        &mut self,
        request: Message<A>,
        rto: Duration,
        rto_cache_duration: Duration,
        queue: &mut TimeoutQueue<TimeoutEntry<A>>,
    ) -> Option<Message<A>> {
        if self.transactions.contains(&request.transaction_id()) {
            queue.push(
                TimeoutEntry::Retransmit {
                    peer: self.peer,
                    request: request.clone(),
                    next_rto: rto * 2,
                },
                rto,
            );
            if self.cached_rto < rto {
                self.cached_rto = rto;
                queue.push(
                    TimeoutEntry::ExpireRtoCache {
                        peer: self.peer,
                        cached_rto: rto,
                    },
                    rto_cache_duration,
                );
            }
            Some(request)
        } else {
            None
        }
    }

    fn start_transaction(&mut self, request: Message<A>) -> (TimeoutEntry<A>, Duration) {
        self.transactions.insert(request.transaction_id());
        self.last_transaction_start_time = SystemTime::now();
        let entry = TimeoutEntry::Retransmit {
            peer: self.peer,
            request,
            next_rto: self.cached_rto * 2,
        };
        (entry, self.cached_rto)
    }

    fn finish_transaction(&mut self, transaction_id: TransactionId) {
        self.transactions.remove(&transaction_id);
    }
}
