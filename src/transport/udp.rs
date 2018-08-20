use fibers::net::futures::{RecvFrom, UdpSocketBind};
use fibers::net::UdpSocket;
use fibers::sync::oneshot::Link;
use fibers::time::timer::{self, Timeout};
use futures::future::Either;
use futures::{Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
use std::cmp;
use std::collections::{BinaryHeap, VecDeque};
use std::fmt;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use trackable::error::ErrorKindExt;

use super::{MessageSink, MessageSinkItem, MessageStream, Transport};
use constants;
use message::{Class, RawMessage};
use {BoxFuture, Error, ErrorKind, Result};

/// `UdpTransport` builder.
#[derive(Debug, Clone)]
pub struct UdpTransportBuilder {
    socket: ::std::result::Result<UdpSocket, SocketAddr>,
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
    recv_buffer_size: usize,
}
impl UdpTransportBuilder {
    /// Makes a new `UdpTransportBuilder` instance with the default settings.
    pub fn new() -> Self {
        let bind_addr = "0.0.0.0:0".parse().unwrap();
        UdpTransportBuilder {
            socket: Err(bind_addr),
            rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(constants::DEFAULT_RTO_CACHE_DURATION_MS),
            min_transaction_interval: Duration::from_millis(
                constants::DEFAULT_MIN_TRANSACTION_INTERVAL_MS,
            ),
            max_outstanding_transactions: constants::DEFAULT_MAX_OUTSTANDING_TRANSACTIONS,
            recv_buffer_size: constants::DEFAULT_MAX_MESSAGE_SIZE,
        }
    }

    /// Makes a new `UdpTransportBuilder` instance with `socket`.
    pub fn with_socket(socket: UdpSocket) -> Self {
        UdpTransportBuilder {
            socket: Ok(socket),
            ..Self::new()
        }
    }

    /// Sets the bind address of this UDP socket.
    ///
    /// The default address is "0.0.0.0:0".
    pub fn bind_addr(&mut self, addr: SocketAddr) -> &mut Self {
        self.socket = Err(addr);
        self
    }

    /// Sets the initial RTO (retransmission timeout).
    ///
    /// The default value is [DEFAULT_RTO_MS](../constants/constant.DEFAULT_RTO_MS.html).
    pub fn rto(&mut self, rto: Duration) -> &mut Self {
        self.rto = rto;
        self
    }

    /// Sets the cache duration of a RTO.
    ///
    /// The default value is [DEFAULT_RTO_CACHE_DURATION_MS]
    /// (../constants/constant.DEFAULT_RTO_CACHE_DURATION_MS.html).
    pub fn rto_cache_duration(&mut self, duration: Duration) -> &mut Self {
        self.rto_cache_duration = duration;
        self
    }

    /// Sets the minimum interval between issuing two consecutive transactions.
    ///
    /// The default value is [DEFAULT_RTO_CACHE_DURATION_MS]
    /// (../constants/constant.DEFAULT_RTO_CACHE_DURATION_MS.html).
    pub fn min_transaction_interval(&mut self, duration: Duration) -> &mut Self {
        self.min_transaction_interval = duration;
        self
    }

    /// Sets the number of maximum outstanding transactions.
    ///
    /// The default value is [DEFAULT_MAX_OUTSTANDING_TRANSACTIONS]
    /// (../constants/constant.DEFAULT_MAX_OUTSTANDING_TRANSACTIONS.html).
    pub fn max_outstanding_transactions(&mut self, count: usize) -> &mut Self {
        self.max_outstanding_transactions = count;
        self
    }

    /// Sets the size of the receiving buffer.
    ///
    /// If a message that has more than `size` is sent, it will discard silently.
    ///
    /// The default value is [DEFAULT_MAX_MESSAGE_SIZE]
    /// (../constants/constant.DEFAULT_MAX_MESSAGE_SIZE.html).
    pub fn recv_buffer_size(&mut self, size: usize) -> &mut Self {
        self.recv_buffer_size = size;
        self
    }

    /// Builds a future which result in a `UdpTransport` instance.
    pub fn finish(&self) -> UdpTransport {
        UdpTransport::from_builder(self)
    }
}

#[derive(Debug, Clone)]
struct SinkParams {
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}

#[derive(Debug)]
struct UdpTransportBind {
    future: UdpSocketBind,
    recv_buffer_size: usize,
    sink_params: SinkParams,
}
impl Future for UdpTransportBind {
    type Item = (UdpMessageSink, UdpMessageStream);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_try!(self.future.poll()).map(|socket| {
            let sink = UdpMessageSink::new(socket.clone(), self.sink_params.clone());
            let stream = UdpMessageStream::new(socket, vec![0; self.recv_buffer_size]);
            (sink, stream)
        }))
    }
}

#[derive(Debug)]
enum UdpTransportInner {
    Binding {
        bind: UdpTransportBind,
        queue: VecDeque<MessageSinkItem>,
    },
    Binded {
        sink: UdpMessageSink,
        stream: UdpMessageStream,
    },
}

/// UDP based implementation of [Transport](trait.Transport.html) trait.
#[derive(Debug)]
pub struct UdpTransport(UdpTransportInner);
impl Transport for UdpTransport {}
impl MessageSink for UdpTransport {}
impl MessageStream for UdpTransport {}
impl UdpTransport {
    /// Makes a new `UdpTransport` instance with the default settings.
    ///
    /// If you want to customize settings of `UdpTransport`,
    /// please use `UdpTransportBuilder` instead.
    pub fn new() -> Self {
        Self::from_builder(&UdpTransportBuilder::new())
    }

    fn from_builder(builder: &UdpTransportBuilder) -> Self {
        let sink_params = SinkParams {
            rto: builder.rto,
            rto_cache_duration: builder.rto_cache_duration,
            min_transaction_interval: builder.min_transaction_interval,
            max_outstanding_transactions: builder.max_outstanding_transactions,
        };
        let inner = match builder.socket.clone() {
            Err(bind_addr) => UdpTransportInner::Binding {
                bind: UdpTransportBind {
                    future: UdpSocket::bind(bind_addr),
                    recv_buffer_size: builder.recv_buffer_size,
                    sink_params: sink_params,
                },
                queue: VecDeque::new(),
            },
            Ok(socket) => {
                let sink = UdpMessageSink::new(socket.clone(), sink_params);
                let stream = UdpMessageStream::new(socket, vec![0; builder.recv_buffer_size]);
                UdpTransportInner::Binded {
                    sink: sink,
                    stream: stream,
                }
            }
        };
        UdpTransport(inner)
    }
    fn poll_bind_complete(&mut self) -> Result<()> {
        let next = match self.0 {
            UdpTransportInner::Binded { .. } => return Ok(()),
            UdpTransportInner::Binding {
                ref mut bind,
                ref mut queue,
            } => {
                if let Async::Ready((mut sink, stream)) = track_try!(bind.poll()) {
                    for item in queue.drain(..) {
                        let started = track_try!(sink.start_send(item));
                        if let AsyncSink::NotReady((_, _, Some(link))) = started {
                            let e = ErrorKind::Other.cause(format!("Sink is full"));
                            link.exit(Err(track!(e).into()));
                        }
                    }
                    UdpTransportInner::Binded {
                        sink: sink,
                        stream: stream,
                    }
                } else {
                    return Ok(());
                }
            }
        };
        self.0 = next;
        Ok(())
    }
}
impl Sink for UdpTransport {
    type SinkItem = MessageSinkItem;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match self.0 {
            UdpTransportInner::Binding {
                ref mut queue,
                ref bind,
            } => {
                if queue.len() >= bind.sink_params.max_outstanding_transactions {
                    Ok(AsyncSink::NotReady(item))
                } else {
                    queue.push_back(item);
                    Ok(AsyncSink::Ready)
                }
            }
            UdpTransportInner::Binded { ref mut sink, .. } => track_err!(sink.start_send(item)),
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        track_try!(self.poll_bind_complete());
        match self.0 {
            UdpTransportInner::Binding { ref queue, .. } => {
                let ready = if queue.is_empty() {
                    Async::Ready(())
                } else {
                    Async::NotReady
                };
                Ok(ready)
            }
            UdpTransportInner::Binded { ref mut sink, .. } => track_err!(sink.poll_complete()),
        }
    }
}
impl Stream for UdpTransport {
    type Item = (SocketAddr, Result<RawMessage>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        track_try!(self.poll_bind_complete());
        match self.0 {
            UdpTransportInner::Binding { .. } => Ok(Async::NotReady),
            UdpTransportInner::Binded { ref mut stream, .. } => track_err!(stream.poll()),
        }
    }
}

#[derive(Debug)]
struct UdpMessageStream(RecvFrom<Vec<u8>>);
impl UdpMessageStream {
    pub fn new(socket: UdpSocket, buf: Vec<u8>) -> Self {
        UdpMessageStream(socket.recv_from(buf))
    }
}
impl Stream for UdpMessageStream {
    type Item = (SocketAddr, Result<RawMessage>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let polled = track_try!(self.0.poll().map_err(|(_, _, e)| e));
        if let Async::Ready((socket, buf, size, peer)) = polled {
            let result = track!(RawMessage::read_from(&mut &buf[..size]));
            let result = result.map_err(|e| {
                let bytes = Vec::from(&buf[..size]);
                ErrorKind::NotStun(bytes).takes_over(e).into()
            });
            self.0 = socket.recv_from(buf);
            Ok(Async::Ready(Some((peer, result))))
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl MessageStream for UdpMessageStream {}

struct UdpMessageSink {
    socket: Either<Option<UdpSocket>, BoxFuture<(UdpSocket, SendItem), Error>>,
    rto_cache: Option<RtoCache>,
    last_transaction_start_time: Option<SystemTime>,
    queue: BinaryHeap<SendItem>,
    params: SinkParams,
}
impl UdpMessageSink {
    fn new(socket: UdpSocket, params: SinkParams) -> Self {
        UdpMessageSink {
            socket: Either::A(Some(socket)),
            rto_cache: None,
            last_transaction_start_time: None,
            queue: BinaryHeap::new(),
            params: params,
        }
    }
    pub fn outstanding_transactions(&self) -> usize {
        if let Either::A(_) = self.socket {
            self.queue.len()
        } else {
            self.queue.len() + 1
        }
    }
    fn drop_rto_cache_if_expired(&mut self) {
        if self
            .rto_cache
            .as_ref()
            .map_or(false, |c| c.expiry_time <= SystemTime::now())
        {
            self.rto_cache = None;
        }
    }
    fn update_rto_cache_if_needed(&mut self, rto: Duration) {
        if self.rto_cache.as_ref().map_or(true, |c| c.rto < rto) {
            self.rto_cache = Some(RtoCache {
                rto: rto,
                expiry_time: SystemTime::now() + self.params.rto_cache_duration,
            });
        }
    }
    fn calc_next_rto(&mut self, class: Class) -> Option<Duration> {
        if class == Class::Request {
            self.drop_rto_cache_if_expired();
            Some(self.rto_cache.as_ref().map_or(self.params.rto, |c| c.rto))
        } else {
            None
        }
    }
    fn calc_next_transaction_wait(
        &mut self,
        class: Class,
    ) -> Result<Option<(SystemTime, Timeout)>> {
        if class == Class::SuccessResponse || class == Class::ErrorResponse {
            return Ok(None);
        }

        let last = if let Some(last) = self.last_transaction_start_time {
            last
        } else {
            return Ok(None);
        };
        let now = SystemTime::now();
        self.last_transaction_start_time = Some(now);

        let passed_time = track_try!(now.duration_since(last));
        if passed_time >= self.params.min_transaction_interval {
            return Ok(None);
        }

        let duration = self.params.min_transaction_interval - passed_time;
        let expiry_time = SystemTime::now() + duration;
        let timeout = timer::timeout(duration);
        Ok(Some((expiry_time, timeout)))
    }
}
impl Sink for UdpMessageSink {
    type SinkItem = MessageSinkItem;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        if self.outstanding_transactions() < self.params.max_outstanding_transactions {
            let (peer, message, link) = item;
            let class = message.class();
            let rto = self.calc_next_rto(class);
            let wait = track_try!(self.calc_next_transaction_wait(class));
            let send_item = SendItem {
                wait: wait,
                peer: peer,
                message: message,
                rto: rto,
                link: link,
            };
            self.queue.push(send_item);
            Ok(AsyncSink::Ready)
        } else {
            Ok(AsyncSink::NotReady(item))
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let socket = match self.socket {
            Either::A(_) => None,
            Either::B(ref mut future) => {
                if let Async::Ready((socket, item)) = track_try!(future.poll()) {
                    Some((socket, item))
                } else {
                    return Ok(Async::NotReady);
                }
            }
        };
        if let Some((socket, mut item)) = socket {
            if let Some(rto) = item.rto {
                if let Ok(Async::NotReady) = item.link.poll() {
                    let rto = rto * 2;
                    let wait = (SystemTime::now() + rto, timer::timeout(rto));
                    self.update_rto_cache_if_needed(rto);
                    item.rto = Some(rto);
                    item.wait = Some(wait);
                    self.queue.push(item);
                }
            }
            self.socket = Either::A(Some(socket));
        }
        if let Some(mut item) = self.queue.pop() {
            if let Async::Ready(()) = track_try!(item.poll()) {
                let socket = if let Either::A(ref mut socket) = self.socket {
                    socket.take().unwrap()
                } else {
                    unreachable!()
                };
                let future = socket.send_to(item.message.to_bytes(), item.peer);
                let future = track_err!(future.map_err(|(_, _, e)| e));
                let future = future.and_then(move |(socket, bytes, sent_size)| {
                    track_assert_eq!(bytes.len(), sent_size, ErrorKind::Other);
                    Ok((socket, item))
                });
                self.socket = Either::B(Box::new(future));
                self.poll_complete()
            } else {
                self.queue.push(item);
                Ok(Async::NotReady)
            }
        } else {
            Ok(Async::Ready(()))
        }
    }
}
impl MessageSink for UdpMessageSink {}
impl fmt::Debug for UdpMessageSink {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UdpMessageSink {{ socket: ")?;
        match self.socket {
            Either::A(ref a) => write!(f, "{:?}, ", a)?,
            Either::B(_) => write!(f, "BoxFuture {{ .. }}, ")?,
        }
        write!(
            f,
            "rto_cache: {:?}, last_transaction_start_time: {:?}, queue: {:?}, params: {:?} }}",
            self.rto_cache, self.last_transaction_start_time, self.queue, self.params
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct RtoCache {
    rto: Duration,
    expiry_time: SystemTime,
}

#[derive(Debug)]
struct SendItem {
    wait: Option<(SystemTime, Timeout)>,
    peer: SocketAddr,
    message: RawMessage,
    rto: Option<Duration>,
    link: Option<Link<(), Error, (), ()>>,
}
impl PartialOrd for SendItem {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        other
            .wait
            .as_ref()
            .map(|t| &t.0)
            .partial_cmp(&self.wait.as_ref().map(|t| &t.0))
    }
}
impl Ord for SendItem {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other
            .wait
            .as_ref()
            .map(|t| &t.0)
            .cmp(&self.wait.as_ref().map(|t| &t.0))
    }
}
impl PartialEq for SendItem {
    fn eq(&self, other: &Self) -> bool {
        self.wait.as_ref().map(|t| &t.0) == other.wait.as_ref().map(|t| &t.0)
    }
}
impl Eq for SendItem {}
impl Future for SendItem {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some((_, ref mut timeout)) = self.wait {
            track_err!(timeout.poll())
        } else {
            Ok(Async::Ready(()))
        }
    }
}
