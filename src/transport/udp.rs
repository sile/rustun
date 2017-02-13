use std::mem;
use std::time::Duration;
use std::net::SocketAddr;
use slog::Logger;
use fibers::net::UdpSocket;
use fibers::net::futures::{RecvFrom, SendTo};
use fibers::time::timer::{self, Timeout};
use futures::{Future, Poll, Async};

use {Result, Error};
use message::RawMessage;
use constants;
use super::{RecvMessage, SendMessage};

#[derive(Debug)]
pub struct UdpSender {
    socket: UdpSocket,
    peer: SocketAddr,
    rto: Duration,
    rc: u32,
    rm: u32,
}
impl UdpSender {
    pub fn new(socket: UdpSocket, peer: SocketAddr) -> Self {
        UdpSender {
            socket: socket,
            peer: peer,
            rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rc: constants::DEFAULT_RC,
            rm: constants::DEFAULT_RM,
        }
    }
    pub fn set_rto(&mut self, rto: Duration) -> &mut Self {
        self.rto = rto;
        self
    }
    pub fn set_rc(&mut self, rc: u32) -> &mut Self {
        self.rc = rc;
        self
    }
    pub fn set_rm(&mut self, rm: u32) -> &mut Self {
        self.rm = rm;
        self
    }
}
impl SendMessage for UdpSender {
    type Future = UdpSendMessage;
    fn send_message(&mut self, message: RawMessage) -> Self::Future {
        UdpSendMessage::new(self, message)
    }
}

#[derive(Debug)]
pub struct UdpSendMessage {
    message: Result<Vec<u8>>,
    send_count: u32,
    timeout: Timeout,
    sender: UdpSender,
    future: Option<SendTo<Vec<u8>>>,
}
impl UdpSendMessage {
    fn new(sender: &UdpSender, message: RawMessage) -> Self {
        let mut buf = Vec::new();
        let result = may_fail!(message.write_to(&mut buf));
        UdpSendMessage {
            message: result.map(|_| buf),
            send_count: 0,
            timeout: timer::timeout(Duration::from_millis(0)),
            sender: UdpSender {
                socket: sender.socket.clone(),
                peer: sender.peer,
                rto: sender.rto,
                rc: sender.rc,
                rm: sender.rm,
            },
            future: None,
        }
    }
}
impl Future for UdpSendMessage {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if let Some(ref mut future) = self.future {
                may_fail!(future.poll().map_err(|(_, _, e)| Error::from(e)))?;
            } else if self.send_count < self.sender.rc {
                let bytes = match mem::replace(&mut self.message, Ok(Vec::new())) {
                    Err(e) => return Err(e),
                    Ok(bytes) => {
                        self.message = Ok(bytes.clone());
                        bytes
                    }
                };
                let future = self.sender.socket.clone().send_to(bytes, self.sender.peer);
                self.send_count += 1;
                let duration = if self.send_count == self.sender.rc {
                    self.sender.rto * self.sender.rm
                } else {
                    self.sender.rto * self.send_count
                };
                self.timeout = timer::timeout(duration);
                self.future = Some(future);
                continue;
            } else {
                return Err(Error::Timeout);
            }
            if let Async::Ready(()) = may_fail!(self.timeout.poll().map_err(Error::failed))? {
                self.future = None;
            } else {
                return Ok(Async::NotReady)
            }
        }
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
        let result = may_fail!(self.future.poll().map_err(|(_, _, e)| Error::from(e)))?;
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
