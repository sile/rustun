use std::time::{SystemTime, Duration};
use std::net::SocketAddr;
use std::sync::mpsc as std_mpsc;
use slog::Logger;
use fibers::net::UdpSocket;
use fibers::net::futures::{RecvFrom, SendTo};
use fibers::time::timer::{self, Timeout};
use futures::{self, Future, Poll, Async, Fuse};
use trackable::error::ErrorKindExt;

use {Error, ErrorKind};
use message::RawMessage;
use constants;
use super::{RecvMessage, SendMessage};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UdpRetransmissionSpec {
    pub rto: Duration,
    pub rto_cache_duration: Duration,
    pub rc: u32,
    pub rm: u32,
}
impl Default for UdpRetransmissionSpec {
    fn default() -> Self {
        UdpRetransmissionSpec {
            rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(constants::DEFAULT_RTO_CACHE_DURATION_MS),
            rc: constants::DEFAULT_RC,
            rm: constants::DEFAULT_RM,
        }
    }
}

#[derive(Debug)]
struct RtoCache {
    rto: Duration,
    expiry_time: SystemTime,
}

#[derive(Debug)]
pub struct UdpSender {
    socket: UdpSocket,
    peer: SocketAddr,
    retransmission_spec: UdpRetransmissionSpec,
    rto_cache: Option<RtoCache>,
    rto_rx: std_mpsc::Receiver<RtoCache>,
    rto_tx: std_mpsc::Sender<RtoCache>,
}
impl UdpSender {
    pub fn new(socket: UdpSocket, peer: SocketAddr) -> Self {
        let (rto_tx, rto_rx) = std_mpsc::channel();
        UdpSender {
            socket: socket,
            peer: peer,
            retransmission_spec: UdpRetransmissionSpec::default(),
            rto_cache: None,
            rto_rx: rto_rx,
            rto_tx: rto_tx,
        }
    }
    pub fn set_retransmission_spec(&mut self, spec: UdpRetransmissionSpec) -> &mut Self {
        self.retransmission_spec = spec;
        self.rto_cache = None;
        self
    }
    pub fn retransmission_spec(&self) -> &UdpRetransmissionSpec {
        &self.retransmission_spec
    }
    pub fn rto_cache(&self) -> Option<Duration> {
        self.rto_cache.as_ref().map(|c| c.rto)
    }
    fn handle_rto_cache(&mut self) {
        let is_expired =
            self.rto_cache.as_ref().map_or(false, |c| c.expiry_time <= SystemTime::now());
        if is_expired {
            self.rto_cache = None;
        }

        while let Ok(cache) = self.rto_rx.try_recv() {
            if self.rto_cache.as_ref().map_or(true, |c| c.rto < cache.rto) {
                self.rto_cache = Some(cache);
            }
        }
    }
}
impl SendMessage for UdpSender {
    type Future = UdpSendMessage;
    fn send_message(&mut self, message: RawMessage) -> Self::Future {
        UdpSendMessage::new(self.socket.clone(), self.peer, message, &self.rto_tx, None)
    }
    fn send_request(&mut self, message: RawMessage) -> Self::Future {
        self.handle_rto_cache();
        let mut spec = self.retransmission_spec.clone();
        if let Some(ref cache) = self.rto_cache {
            spec.rto = cache.rto;
        }
        UdpSendMessage::new(self.socket.clone(),
                            self.peer,
                            message,
                            &self.rto_tx,
                            Some(spec))
    }
}

enum SendInner {
    Call(Call),
    Cast(Cast),
    Failed(futures::Failed<(), Error>),
}
impl Future for SendInner {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            SendInner::Call(ref mut f) => f.poll(),
            SendInner::Cast(ref mut f) => f.poll(),
            SendInner::Failed(ref mut f) => f.poll(),
        }
    }
}

// #[derive(Debug)]
struct Call {
    socket: UdpSocket,
    peer: SocketAddr,
    message: Vec<u8>,
    send_count: u32,
    retransmission_spec: UdpRetransmissionSpec,
    rto_tx: std_mpsc::Sender<RtoCache>,
    timeout: Timeout,
    future: Option<Fuse<SendTo<Vec<u8>>>>,
}
impl Drop for Call {
    fn drop(&mut self) {
        if self.send_count > 1 {
            let rto = self.retransmission_spec.rto * self.send_count;
            let expiry_time = SystemTime::now() + self.retransmission_spec.rto_cache_duration;
            let cache = RtoCache {
                rto: rto,
                expiry_time: expiry_time,
            };
            let _ = self.rto_tx.send(cache);
        }
    }
}
impl Future for Call {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if let Some(ref mut future) = self.future {
                track_try!(future.poll().map_err(|(_, _, e)| e));
            } else if self.send_count < self.retransmission_spec.rc {
                let bytes = self.message.clone();
                let future = self.socket.clone().send_to(bytes, self.peer);
                self.send_count += 1;
                let duration = if self.send_count == self.retransmission_spec.rc {
                    self.retransmission_spec.rto * self.retransmission_spec.rm
                } else {
                    self.retransmission_spec.rto * self.send_count
                };
                self.timeout = timer::timeout(duration);
                self.future = Some(future.fuse());
                continue;
            } else {
                return Err(ErrorKind::Timeout.into());
            }
            if let Async::Ready(()) = track_err!(self.timeout.poll().map_err(|e| ErrorKind::Failed.cause(e)))? {
                self.future = None;
            } else {
                return Ok(Async::NotReady)
            }
        }
    }
}

#[derive(Debug)]
struct Cast(SendTo<Vec<u8>>);
impl Future for Cast {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        track_err!(self.0.poll().map(|r| r.map(|_| ())).map_err(|(_, _, e)| ErrorKind::Failed.cause(e)))
    }
}

pub struct UdpSendMessage(SendInner);
impl UdpSendMessage {
    fn new(socket: UdpSocket,
           peer: SocketAddr,
           message: RawMessage,
           rto_tx: &std_mpsc::Sender<RtoCache>,
           retransmission_spec: Option<UdpRetransmissionSpec>)
           -> Self {
        let mut buf = Vec::new();
        let inner = match track_err!(message.write_to(&mut buf)) {
            Err(e) => SendInner::Failed(futures::failed(e)),
            Ok(_) => {
                if let Some(retransmission_spec) = retransmission_spec {
                    SendInner::Call(Call {
                        socket: socket,
                        peer: peer,
                        message: buf,
                        timeout: timer::timeout(Duration::from_millis(0)),
                        send_count: 0,
                        retransmission_spec: retransmission_spec,
                        rto_tx: rto_tx.clone(),
                        future: None,
                    })
                } else {
                    SendInner::Cast(Cast(socket.send_to(buf, peer)))
                }
            }
        };
        UdpSendMessage(inner)
    }
}
impl Future for UdpSendMessage {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

#[derive(Debug)]
pub struct UdpReceiver {
    socket: UdpSocket,
    buffer: Vec<u8>,
    logger: Option<Logger>,
}
impl UdpReceiver {
    pub fn new(socket: UdpSocket) -> Self {
        Self::with_buffer(socket, vec![0; constants::DEFAULT_MAX_MESSAGE_SIZE])
    }
    pub fn with_buffer(socket: UdpSocket, buffer: Vec<u8>) -> Self {
        UdpReceiver {
            socket: socket,
            buffer: buffer,
            logger: None,
        }
    }
    pub fn set_logger(&mut self, logger: Logger) {
        self.logger = Some(logger);
    }
}
impl RecvMessage for UdpReceiver {
    type Future = UdpRecvMessage;
    fn recv_message(self) -> Self::Future {
        let UdpReceiver { socket, buffer, logger } = self;
        UdpRecvMessage {
            logger: logger,
            future: socket.recv_from(buffer),
        }
    }
}

#[derive(Debug)]
pub struct UdpRecvMessage {
    logger: Option<Logger>,
    future: RecvFrom<Vec<u8>>,
}
impl Future for UdpRecvMessage {
    type Item = (UdpReceiver, SocketAddr, RawMessage);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let result = track_try!(self.future.poll().map_err(|(_, _, e)| e));
        if let Async::Ready((socket, buf, size, peer)) = result {
            match RawMessage::read_from(&mut &buf[..size]) {
                Err(e) => {
                    if let Some(ref logger) = self.logger {
                        info!(logger,
                              "Cannot decode STUN message from {}: reason={}",
                              peer,
                              e);
                    }
                    self.future = socket.recv_from(buf);
                    self.poll()
                }
                Ok(message) => {
                    let receiver = UdpReceiver {
                        logger: self.logger.take(),
                        socket: socket,
                        buffer: buf,
                    };
                    Ok(Async::Ready((receiver, peer, message)))
                }
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}
