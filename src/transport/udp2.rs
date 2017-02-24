use std::cmp;
use std::net::SocketAddr;
use std::collections::BinaryHeap;
use std::time::{SystemTime, Duration};
use fibers::Spawn;
use fibers::net::UdpSocket;
use fibers::net::futures::{UdpSocketBind, RecvFrom};
use fibers::sync::oneshot::Link;
use fibers::time::timer::{self, Timeout};
use futures::{BoxFuture, Future, Stream, Poll, Async, StartSend, Sink, AsyncSink};
use futures::future::Either;

use {Result, Error, ErrorKind};
use message::{Class, RawMessage};
use constants;
use super::{MessageStream, MessageSink, MessageSinkItem, Transport};

#[derive(Debug, Clone)]
pub struct UdpTransportBuilder {
    bind_addr: SocketAddr,
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
    recv_buffer_size: usize,
}
impl UdpTransportBuilder {
    pub fn new() -> Self {
        UdpTransportBuilder {
            bind_addr: "0.0.0.0:0".parse().unwrap(),
            rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(constants::DEFAULT_RTO_CACHE_DURATION_MS),
            min_transaction_interval:
                Duration::from_millis(constants::DEFAULT_MIN_TRANSACTION_INTERVAL_MS),
            max_outstanding_transactions: constants::DEFAULT_MAX_CLIENT_CONCURRENCY,
            recv_buffer_size: constants::DEFAULT_MAX_MESSAGE_SIZE,
        }
    }
    pub fn bind_addr(&mut self, addr: SocketAddr) -> &mut Self {
        self.bind_addr = addr;
        self
    }
    pub fn rto(&mut self, rto: Duration) -> &mut Self {
        self.rto = rto;
        self
    }
    pub fn rto_cache_duration(&mut self, duration: Duration) -> &mut Self {
        self.rto_cache_duration = duration;
        self
    }
    pub fn min_transaction_interval(&mut self, duration: Duration) -> &mut Self {
        self.min_transaction_interval = duration;
        self
    }
    pub fn max_outstanding_transactions(&mut self, count: usize) -> &mut Self {
        self.max_outstanding_transactions = count;
        self
    }
    pub fn recv_buffer_size(&mut self, size: usize) -> &mut Self {
        self.recv_buffer_size = size;
        self
    }
    pub fn finish<S: Spawn>(&self, spawner: &S) -> UdpTransportBind {
        let sink_params = SinkParams {
            rto: self.rto,
            rto_cache_duration: self.rto_cache_duration,
            min_transaction_interval: self.min_transaction_interval,
            max_outstanding_transactions: self.max_outstanding_transactions,
        };
        UdpTransportBind {
            future: UdpSocket::bind(self.bind_addr),
            recv_buffer_size: self.recv_buffer_size,
            sink_params: sink_params,
        }
    }
}

#[derive(Clone)]
struct SinkParams {
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}

pub struct UdpTransportBind {
    future: UdpSocketBind,
    recv_buffer_size: usize,
    sink_params: SinkParams,
}
impl Future for UdpTransportBind {
    type Item = UdpTransport;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_try!(self.future.poll()).map(|socket| {
            UdpTransport::new(socket, self.recv_buffer_size, self.sink_params.clone())
        }))
    }
}

pub struct UdpTransport {
    sink: UdpMessageSink,
    stream: UdpMessageStream,
}
impl Transport for UdpTransport {}
impl MessageSink for UdpTransport {}
impl MessageStream for UdpTransport {}
impl UdpTransport {
    fn new(socket: UdpSocket, recv_buffer_size: usize, sink_params: SinkParams) -> Self {
        UdpTransport {
            sink: UdpMessageSink::new(socket.clone(), sink_params),
            stream: UdpMessageStream::new(socket, vec![0; recv_buffer_size]),
        }
    }
}
impl Sink for UdpTransport {
    type SinkItem = MessageSinkItem;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sink.start_send(item)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.poll_complete()
    }
}
impl Stream for UdpTransport {
    type Item = (SocketAddr, Result<RawMessage>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.stream.poll()
    }
}

pub struct UdpMessageStream(RecvFrom<Vec<u8>>);
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
            let result = track_err!(RawMessage::read_from(&mut &buf[..size]));
            self.0 = socket.recv_from(buf);
            Ok(Async::Ready(Some((peer, result))))
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl MessageStream for UdpMessageStream {}

pub struct UdpMessageSink {
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
        if self.rto_cache.as_ref().map_or(false, |c| c.expiry_time <= SystemTime::now()) {
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
    fn calc_next_transaction_wait(&mut self,
                                  class: Class)
                                  -> Result<Option<(SystemTime, Timeout)>> {
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
                self.socket = Either::B(future.boxed());
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

#[derive(Debug, Clone)]
struct RtoCache {
    rto: Duration,
    expiry_time: SystemTime,
}

struct SendItem {
    wait: Option<(SystemTime, Timeout)>,
    peer: SocketAddr,
    message: RawMessage,
    rto: Option<Duration>,
    link: Link<(), (), (), Error>,
}
impl PartialOrd for SendItem {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        other.wait.as_ref().map(|t| &t.0).partial_cmp(&self.wait.as_ref().map(|t| &t.0))
    }
}
impl Ord for SendItem {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other.wait.as_ref().map(|t| &t.0).cmp(&self.wait.as_ref().map(|t| &t.0))
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
