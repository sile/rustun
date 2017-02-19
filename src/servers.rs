use std::mem;
use std::net::SocketAddr;
use slog::{self, Logger};
use fibers::{Spawn, BoxSpawn};
use fibers::net::UdpSocket;
use fibers::net::futures::UdpSocketBind;
use futures::{Future, Poll, Async, Stream};

use {Result, Error, HandleMessage, Message, ErrorKind, Method};
use message::RawMessage;
use transport::{RecvMessage, UdpReceiver, SendMessage};
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
    pub fn start<H: HandleMessage>(&mut self, spawner: BoxSpawn, handler: H) -> UdpServer<H> {
        let state = UdpServerState {
            logger: self.logger.clone(),
            spawner: spawner,
            handler: handler,
        };
        let future = UdpSocket::bind(self.bind_addr);
        UdpServer(UdpServerInner::Bind(future, state))
    }
}

//#[derive(Debug)]
pub struct UdpServer<H>(UdpServerInner<H>);
impl<H: HandleMessage> Future for UdpServer<H> {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

#[derive(Debug)]
struct UdpServerState<H> {
    logger: Logger,
    spawner: BoxSpawn,
    handler: H,
}

//#[derive(Debug)]
enum UdpServerInner<H> {
    Bind(UdpSocketBind, UdpServerState<H>),
    Loop(UdpServerLoop<H>),
    Done,
}
impl<H: HandleMessage> Future for UdpServerInner<H> {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, UdpServerInner::Done) {
            UdpServerInner::Bind(mut future, state) => {
                if let Async::Ready(socket) = track_try!(future.poll()) {
                    let future = UdpServerLoop::new(socket, state);
                    *self = UdpServerInner::Loop(future);
                    self.poll()
                } else {
                    *self = UdpServerInner::Bind(future, state);
                    Ok(Async::NotReady)
                }
            }
            UdpServerInner::Loop(mut future) =>{
                if let Async::Ready(()) = track_try!(future.poll()) {
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
struct UdpServerLoop<H> {
    socket: UdpSocket,
    stream: MessageStream<UdpReceiver>,
    state: UdpServerState<H>,
}
impl<H: HandleMessage> UdpServerLoop<H> {
    pub fn new(socket: UdpSocket, state: UdpServerState<H>) -> Self {
        UdpServerLoop {
            socket: socket.clone(),
            stream: UdpReceiver::new(socket).into_stream(),
            state: state,
        }
    }
    fn handle_message(&mut self, client: SocketAddr, message: RawMessage) -> Result<()> {
        let message: Message<H::Method, H::Attribute> = track_try!(Message::try_from_raw(message));
        track_assert!(message.is_permitted(),
                      ErrorKind::Failed,
                      "The class {:?} is not permitted by the method {:?}",
                      message.class(),
                      message.method().as_u12());
        if message.class().is_request() {
            let mut sender = ::transport::UdpSender::new(self.socket.clone(), client); // XXX: over spec
            let request = message.try_into_request().unwrap();
            self.state.spawner.spawn(self.state
                .handler
                .handle_call(client, request)
                                     .and_then(move |response| {
                                         println!("# IN RESPONSE");
                    // TODO: handle error
                    let raw = response.into_inner().try_into_raw().unwrap();
                    let future = sender.send_message(raw).map_err(|e| { println!("Error: {}", e); ()});
                    future.then(move |_| {
                        println!("# DONE");
                        let _ = sender; Ok(())})
                }));
        } else if message.class().is_indication() {
            let indication = message.try_into_indication().unwrap();
            self.state.spawner.spawn(self.state.handler.handle_cast(client, indication));
        } else {
            track_panic!(ErrorKind::Failed,
                         "Unexpected message class: {:?}",
                         message.class());
        }
        Ok(())
    }
}
impl<H: HandleMessage> Future for UdpServerLoop<H> {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match track_try!(self.stream.poll()) {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(None) => {
                info!(self.state.logger, "UdpServer terminated");
                Ok(Async::Ready(()))
            }
            Async::Ready(Some((addr, message))) => {
                debug!(self.state.logger, "Recv from {}: {:?}", addr, message);
                if let Err(e) = self.handle_message(addr, message) {
                    warn!(self.state.logger,
                          "Cannot handle a message from {}: {}",
                          addr,
                          e);
                }
                Ok(Async::NotReady)
            }
        }
    }
}
