use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use std::sync::mpsc as std_mpsc;
use fibers::Spawn;
use fibers::net::UdpSocket;
use fibers::net::futures::{UdpSocketBind, RecvFrom};
use fibers::time::timer;
use fibers::sync::oneshot::Monitor;
use futures::{Future, Poll, Async, BoxFuture};
use futures::future::Either;

use {Error, ErrorKind};
use constants;
use transport::Transport;
use message::RawMessage;
use super::FifoRunner;

#[derive(Debug, Clone)]
pub struct UdpTransportBuilder {
    bind_addr: SocketAddr,
    rto: Duration,
    rto_cache_duration: Duration,
    recv_buffer_size: usize,
}
impl UdpTransportBuilder {
    pub fn new() -> Self {
        UdpTransportBuilder {
            bind_addr: "0.0.0.0:0".parse().unwrap(),
            rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(constants::DEFAULT_RTO_CACHE_DURATION_MS),
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
    pub fn recv_buffer_size(&mut self, size: usize) -> &mut Self {
        self.recv_buffer_size = size;
        self
    }
    pub fn finish<S: Spawn>(&self, spawner: &S) -> UdpTransportBind {
        UdpTransportBind {
            future: UdpSocket::bind(self.bind_addr),
            runner: FifoRunner::new(spawner),
            params: self.clone(),
        }
    }
}

pub struct UdpTransportBind {
    future: UdpSocketBind,
    runner: FifoRunner<BoxFuture<UdpSocket, Error>>,
    params: UdpTransportBuilder,
}
impl Future for UdpTransportBind {
    type Item = UdpTransport;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_try!(self.future.poll())
            .map(|socket| UdpTransport::new(socket, self.runner.clone(), self.params.clone())))
    }
}

#[derive(Debug, Clone)]
struct RtoCache {
    rto: Duration,
    expiry_time: SystemTime,
}

pub struct UdpTransport {
    socket: UdpSocket,
    runner: FifoRunner<BoxFuture<UdpSocket, Error>>,
    params: UdpTransportBuilder,
    rto_cache: Option<RtoCache>,
    rto_cache_tx: std_mpsc::Sender<Duration>,
    rto_cache_rx: std_mpsc::Receiver<Duration>,
}
impl UdpTransport {
    fn new(socket: UdpSocket,
           runner: FifoRunner<BoxFuture<UdpSocket, Error>>,
           params: UdpTransportBuilder)
           -> Self {
        let (rto_cache_tx, rto_cache_rx) = std_mpsc::channel();
        UdpTransport {
            socket: socket,
            runner: runner,
            params: params,
            rto_cache: None,
            rto_cache_tx: rto_cache_tx,
            rto_cache_rx: rto_cache_rx,
        }
    }
    fn update_rto_cache(&mut self) {
        if let Ok(rto) = self.rto_cache_rx.try_recv() {
            let expiry_time = SystemTime::now() + self.params.rto_cache_duration;
            self.rto_cache = Some(RtoCache {
                rto: rto,
                expiry_time: expiry_time,
            });
        } else if let Some(cache) = self.rto_cache.clone() {
            if cache.expiry_time <= SystemTime::now() {
                self.rto_cache = None;
            }
        }
    }
    fn rto(&self) -> Duration {
        self.rto_cache.as_ref().map_or(self.params.rto, |c| c.rto)
    }
}
impl Transport for UdpTransport {
    type SendMessage = UdpSendMessage;
    type RecvMessage = UdpRecvMessage;
    fn send_message(&mut self, peer: SocketAddr, message: RawMessage) -> Self::SendMessage {
        self.update_rto_cache();
        UdpSendMessage::new(self, peer, message)
    }
    fn recv_message(&mut self) -> Self::RecvMessage {
        UdpRecvMessage::new(self)
    }
}

pub struct UdpSendMessage {
    peer: SocketAddr,
    runner: FifoRunner<BoxFuture<UdpSocket, Error>>,
    start_rto: Duration,
    current_rto: Duration,
    rto_cache_tx: std_mpsc::Sender<Duration>,
    message: RawMessage,
    state: Either<Monitor<UdpSocket, Error>, BoxFuture<UdpSocket, Error>>,
}
impl UdpSendMessage {
    fn new(transport: &UdpTransport, peer: SocketAddr, message: RawMessage) -> Self {
        let socket = transport.socket.clone();
        let rto = transport.rto();
        let future = transport.runner.register(Self::send_message(socket, &message, peer));
        UdpSendMessage {
            peer: peer,
            runner: transport.runner.clone(),
            start_rto: rto,
            current_rto: rto,
            rto_cache_tx: transport.rto_cache_tx.clone(),
            message: message,
            state: Either::A(future),
        }
    }
    fn send_message(socket: UdpSocket,
                    message: &RawMessage,
                    peer: SocketAddr)
                    -> BoxFuture<UdpSocket, Error> {
        track_err!(socket.send_to(message.to_bytes(), peer).map_err(|(_, _, e)| e))
            .and_then(|(socket, bytes, sent_size)| {
                track_assert_eq!(sent_size, bytes.len(), ErrorKind::Other);
                Ok(socket)
            })
            .boxed()
    }
}
impl Future for UdpSendMessage {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let next_state = match self.state {
            Either::A(ref mut future) => {
                if let Async::Ready(socket) = track_try!(future.poll()) {
                    let rto = self.current_rto;
                    let timeout = track_err!(timer::timeout(rto).map(move |()| socket));
                    self.current_rto = rto * 2;
                    Some(Either::B(timeout.boxed()))
                } else {
                    None
                }
            }
            Either::B(ref mut timeout) => {
                if let Async::Ready(socket) = track_try!(timeout.poll()) {
                    let future = Self::send_message(socket, &self.message, self.peer);
                    Some(Either::A(self.runner.register(future)))
                } else {
                    None
                }
            }
        };
        if let Some(state) = next_state {
            self.state = state;
            self.poll()
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl Drop for UdpSendMessage {
    fn drop(&mut self) {
        if self.start_rto != self.current_rto {
            let _ = self.rto_cache_tx.send(self.current_rto);
        }
    }
}

#[derive(Debug)]
pub struct UdpRecvMessage {
    future: RecvFrom<Vec<u8>>,
}

impl UdpRecvMessage {
    fn new(transport: &UdpTransport) -> Self {
        let buf = vec![0; transport.params.recv_buffer_size];
        UdpRecvMessage { future: transport.socket.clone().recv_from(buf) }
    }
}
impl Future for UdpRecvMessage {
    type Item = (SocketAddr, RawMessage);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let polled = track_try!(self.future.poll().map_err(|(_, _, e)| e));
        if let Async::Ready((_socket, buf, size, peer)) = polled {
            let message = track_try!(RawMessage::read_from(&mut &buf[..size]));
            Ok(Async::Ready((peer, message)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
