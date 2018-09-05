use bytecodec::marker::Never;
use futures::{Async, Future, Poll};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use stun_codec::{Attribute, Message, MessageClass, Method, TransactionId};
use trackable::error::ErrorKindExt;

use super::timeout_queue::TimeoutQueue;
use super::Client;
use constants;
use message::{ErrorResponse, Indication, Request, Response, SuccessResponse};
use transport::{StunTransport, UdpTransport};
use {AsyncReply, Error, ErrorKind, Result};

// TODO: UdpClientBuilder

#[derive(Debug)]
struct TransactionState<M, A> {
    reply: AsyncReply<Response<M, A>>,
    request_method: M,
    started: bool,
}

#[derive(Debug)]
pub struct UdpClient<M, A, T> {
    transporter: T,

    // TODO: s/transactions/requests/
    transactions: HashMap<TransactionId, TransactionState<M, A>>,
    request_timeout: Duration,
    timeout_queue: TimeoutQueue<TimeoutEntry<M, A>>,
    peer: SocketAddr,
    pendings: VecDeque<Request<M, A>>,
    set_wait_timeout: bool,

    cache_rto: Duration,
    initial_rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
    recv_buffer_size: usize,
    last_transaction_start_time: SystemTime,
}
impl<M, A, T> UdpClient<M, A, T>
where
    M: Method,
    A: Attribute,
    T: StunTransport<M, A> + UdpTransport,
{
    pub fn new(transporter: T, peer: SocketAddr) -> Self {
        UdpClient {
            transporter,
            transactions: HashMap::new(),
            request_timeout: Duration::from_millis(constants::DEFAULT_TIMEOUT_MS),
            timeout_queue: TimeoutQueue::new(),
            peer,
            pendings: VecDeque::new(),
            set_wait_timeout: false,

            // TODO:
            cache_rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            initial_rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(constants::DEFAULT_RTO_CACHE_DURATION_MS),
            min_transaction_interval: Duration::from_millis(
                constants::DEFAULT_MIN_TRANSACTION_INTERVAL_MS,
            ),
            max_outstanding_transactions: constants::DEFAULT_MAX_OUTSTANDING_TRANSACTIONS,
            recv_buffer_size: constants::DEFAULT_MAX_MESSAGE_SIZE,

            last_transaction_start_time: UNIX_EPOCH, // sentinel
        }
    }

    /// Sets the timeout duration of a request transaction.
    ///
    /// The default value is [DEFAULT_TIMEOUT_MS](../constants/constant.DEFAULT_TIMEOUT_MS.html).
    pub fn set_request_timeout(&mut self, timeout: Duration) {
        self.request_timeout = timeout;
    }

    fn handle_message(&mut self, message: Message<M, A>) {
        let t = if let Some(t) = self.transactions.remove(message.transaction_id()) {
            t
        } else {
            return;
        };
        t.reply
            .send(track!(self.make_response(t.request_method, message)));
        self.handle_pending_request();
    }

    fn handle_timeout(&mut self, transaction_id: TransactionId) {
        if let Some(t) = self.transactions.remove(&transaction_id) {
            let e = track!(ErrorKind::Timeout.error());
            t.reply.send(Err(e.into()));
        }
        self.handle_pending_request();
    }

    fn make_response(&self, request_method: M, response: Message<M, A>) -> Result<Response<M, A>> {
        track_assert_eq!(
            request_method.as_u12(),
            response.method().as_u12(),
            ErrorKind::InvalidInput
        );

        match response.class() {
            MessageClass::SuccessResponse => {
                track!(SuccessResponse::from_message(response)).map(Ok)
            }
            MessageClass::ErrorResponse => track!(ErrorResponse::from_message(response)).map(Err),
            class => {
                track_panic!(
                    ErrorKind::InvalidInput,
                    "Unexpected class of response message: {:?}",
                    class
                );
            }
        }
    }

    fn poll_expired(&mut self) -> Option<TimeoutEntry<M, A>> {
        let transactions = &self.transactions;
        self.timeout_queue.pop_expired(|entry| {
            entry
                .transaction_id()
                .map_or(true, |id| transactions.contains_key(id))
        })
    }

    fn handle_pending_request(&mut self) {
        if let Some(request) = self.pendings.pop_front() {
            self.start_transaction(request, false);
        }
    }

    fn waiting_time(&self) -> Option<Duration> {
        self.last_transaction_start_time
            .elapsed()
            .ok()
            .and_then(|d| self.min_transaction_interval.checked_sub(d))
    }

    fn pending(&mut self, request: Request<M, A>, first: bool) {
        if first {
            self.pendings.push_back(request);
        } else {
            self.pendings.push_front(request);
        }
    }

    fn start_transaction(&mut self, request: Request<M, A>, first: bool) {
        if self.set_wait_timeout {
            self.pending(request, first);
        } else if let Some(wait) = self.waiting_time() {
            if self.set_wait_timeout {
                self.timeout_queue.push(TimeoutEntry::Wait, wait);
                self.set_wait_timeout = true;
            }
            self.pending(request, first);
        } else if self.outstanding_transactions() >= self.max_outstanding_transactions {
            self.pending(request, first);
        } else {
            self.transactions
                .get_mut(request.transaction_id())
                .expect("never fails")
                .started = true;
            self.transporter
                .send(self.peer, request.clone().into_message());
            self.timeout_queue.push(
                TimeoutEntry::Retransmission(request, self.cache_rto),
                self.cache_rto,
            );
            self.last_transaction_start_time = SystemTime::now();
        }
    }

    fn outstanding_transactions(&self) -> usize {
        self.transactions.values().filter(|t| t.started).count()
    }

    fn handle_retransmission(&mut self, request: Request<M, A>, rto: Duration) {
        self.transporter
            .send(self.peer, request.clone().into_message());
        let new_rto = rto * 2;
        self.timeout_queue
            .push(TimeoutEntry::Retransmission(request, new_rto), new_rto);
        if self.cache_rto < new_rto {
            self.cache_rto = new_rto;
            self.timeout_queue.push(
                TimeoutEntry::ExpireRtoCache(new_rto),
                self.rto_cache_duration,
            );
        }
    }

    fn handle_expire_rto_cache(&mut self, rto: Duration) {
        if self.cache_rto <= rto {
            self.cache_rto = self.initial_rto;
        }
    }
}
impl<M, A, T> Client<M, A> for UdpClient<M, A, T>
where
    M: Method,
    A: Attribute,
    T: StunTransport<M, A> + UdpTransport,
{
    fn call_with_reply(&mut self, request: Request<M, A>, reply: AsyncReply<Response<M, A>>) {
        let tid = request.transaction_id().clone();
        self.transactions.insert(
            tid.clone(),
            TransactionState {
                request_method: request.method().clone(),
                reply,
                started: false,
            },
        );
        self.timeout_queue
            .push(TimeoutEntry::Request(tid.clone()), self.request_timeout);
        self.start_transaction(request, true);
    }

    fn cast(&mut self, indication: Indication<M, A>) {
        self.transporter.send(self.peer, indication.into_message());
    }
}
impl<M, A, T> Future for UdpClient<M, A, T>
where
    M: Method,
    A: Attribute,
    T: StunTransport<M, A> + UdpTransport,
{
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Some((_, message)) = self.transporter.recv() {
            self.handle_message(message);
        }

        while let Some(entry) = self.poll_expired() {
            match entry {
                TimeoutEntry::Request(id) => self.handle_timeout(id),
                TimeoutEntry::Retransmission(request, rto) => {
                    self.handle_retransmission(request, rto)
                }
                TimeoutEntry::ExpireRtoCache(rto) => self.handle_expire_rto_cache(rto),
                TimeoutEntry::Wait => {
                    self.set_wait_timeout = false;
                    self.handle_pending_request();
                }
            }
        }

        if track!(self.transporter.poll_finish())? {
            track_panic!(ErrorKind::Other, "TCP connection closed by peer");
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
enum TimeoutEntry<M, A> {
    Request(TransactionId),
    Retransmission(Request<M, A>, Duration),
    ExpireRtoCache(Duration),
    Wait,
}
impl<M: Method, A: Attribute> TimeoutEntry<M, A> {
    fn transaction_id(&self) -> Option<&TransactionId> {
        match self {
            TimeoutEntry::Request(id) => Some(id),
            TimeoutEntry::Retransmission(request, _) => Some(request.transaction_id()),
            TimeoutEntry::ExpireRtoCache(_) => None,
            TimeoutEntry::Wait => None,
        }
    }
}
