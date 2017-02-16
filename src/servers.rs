use std::mem;
use std::net::SocketAddr;
use slog::{self, Logger};
use fibers::net::UdpSocket;
use fibers::net::futures::UdpSocketBind;
use futures::{Future, Poll, Async, Stream};

use Error;
use transport::{RecvMessage, UdpReceiver};
use transport::streams::MessageStream;

#[derive(Debug)]
pub struct UdpServerBuilder {
    bind_addr: SocketAddr,
    logger: Logger,
}
impl UdpServerBuilder {
    pub fn new(bind_addr: SocketAddr) -> Self {
        UdpServerBuilder {
            bind_addr: bind_addr,
            logger: Logger::root(slog::Discard, o!()),
        }
    }
    pub fn logger(&mut self, logger: Logger) -> &mut Self {
        self.logger = logger;
        self
    }
    pub fn start(&mut self) -> UdpServer {
        let state = UdpServerState { logger: self.logger.clone() };
        let future = UdpSocket::bind(self.bind_addr);
        UdpServer(UdpServerInner::Bind(future, state))
    }
}

//#[derive(Debug)]
pub struct UdpServer(UdpServerInner);
impl Future for UdpServer {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

#[derive(Debug)]
struct UdpServerState {
    logger: Logger,
}

//#[derive(Debug)]
enum UdpServerInner {
    Bind(UdpSocketBind, UdpServerState),
    Loop(UdpServerLoop),
    Done,
}
impl Future for UdpServerInner {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, UdpServerInner::Done) {
            UdpServerInner::Bind(mut future, state) => {
                if let Async::Ready(socket) = may_fail!(future.poll().map_err(Error::from_cause))? {
                    let future = UdpServerLoop::new(socket, state);
                    *self = UdpServerInner::Loop(future);
                    self.poll()
                } else {
                    *self = UdpServerInner::Bind(future, state);
                    Ok(Async::NotReady)
                }
            }
            UdpServerInner::Loop(mut future) =>{
                if let Async::Ready(()) = may_fail!(future.poll())? {
                    Ok(Async::Ready(()))
                } else {
                    *self = UdpServerInner::Loop(future);
                    Ok(Async::NotReady)
                }
            }
            UdpServerInner::Done => panic!("Cannot poll UdpServerInner twice"),
        }
    }
}

// #[derive(Debug)]
struct UdpServerLoop {
    stream: MessageStream<UdpReceiver>,
    state: UdpServerState,
}
impl UdpServerLoop {
    pub fn new(socket: UdpSocket, state: UdpServerState) -> Self {
        UdpServerLoop {
            stream: UdpReceiver::new(socket).into_stream(),
            state: state,
        }
    }
}
impl Future for UdpServerLoop {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match may_fail!(self.stream.poll())? {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(None) => {
                info!(self.state.logger, "UdpServer terminated");
                Ok(Async::Ready(()))
            }
            Async::Ready(Some(message)) => {
                // TODO: handle
                debug!(self.state.logger, "Receives: {:?}", message);
                Ok(Async::NotReady)
            }
        }
    }
}
