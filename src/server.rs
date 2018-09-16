//! Basic STUN servers.
//!
//! This module provides only a basic STUN servers.
//! If you want more elaborate one, please consider create your own server using [`Channel`] directly.
//!
//! [`Channel`]: ../channel/struct.Channel.html
use bytecodec::marker::Never;
use factory::Factory;
use fibers::net::futures::{TcpListenerBind, UdpSocketBind};
use fibers::net::streams::Incoming;
use fibers::net::{TcpListener, UdpSocket};
use fibers::sync::mpsc;
use fibers::{BoxSpawn, Spawn};
use fibers_transport::{TcpTransporter, UdpTransporter};
use futures::future::Either;
use futures::{self, Async, Future, Poll, Stream};
use std::fmt;
use std::net::SocketAddr;
use stun_codec::rfc5389;
use stun_codec::{Attribute, MessageDecoder, MessageEncoder};

use channel::{Channel, RecvMessage};
use message::{ErrorResponse, Indication, InvalidMessage, Request, Response, SuccessResponse};
use transport::{StunTcpTransporter, StunTransport, StunUdpTransporter};
use {Error, ErrorKind, Result};

/// The default TCP and UDP port for STUN.
pub const DEFAULT_PORT: u16 = 3478;

/// The default TLS port for STUN.
pub const DEFAULT_TLS_PORT: u16 = 5349;

/// UDP based STUN server.
#[derive(Debug)]
#[must_use = "future do nothing unless polled"]
pub struct UdpServer<H: HandleMessage>(UdpServerInner<H>);
impl<H: HandleMessage> UdpServer<H> {
    /// Starts the server.
    pub fn start<S>(spawner: S, bind_addr: SocketAddr, handler: H) -> Self
    where
        S: Spawn + Send + 'static,
    {
        UdpServer(UdpServerInner::Binding {
            future: UdpSocket::bind(bind_addr),
            spawner: Some(spawner.boxed()),
            handler: Some(handler),
        })
    }
}
impl<H: HandleMessage> Future for UdpServer<H> {
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

enum UdpServerInner<H: HandleMessage> {
    Binding {
        future: UdpSocketBind,
        spawner: Option<BoxSpawn>,
        handler: Option<H>,
    },
    Running {
        driver: HandlerDriver<
            H,
            StunUdpTransporter<
                H::Attribute,
                UdpTransporter<MessageEncoder<H::Attribute>, MessageDecoder<H::Attribute>>,
            >,
        >,
    },
}
impl<H: HandleMessage> Future for UdpServerInner<H> {
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next = match self {
                UdpServerInner::Binding {
                    future,
                    spawner,
                    handler,
                } => {
                    if let Async::Ready(socket) = track!(future.poll().map_err(Error::from))? {
                        let transporter =
                            StunUdpTransporter::new(track!(UdpTransporter::from_socket(socket))?);
                        let channel = Channel::new(transporter);
                        let driver = HandlerDriver::new(
                            spawner.take().expect("never fails"),
                            handler.take().expect("never fails"),
                            channel,
                        );
                        UdpServerInner::Running { driver }
                    } else {
                        break;
                    }
                }
                UdpServerInner::Running { driver } => {
                    if let Async::Ready(()) = track!(driver.poll())? {
                        track_panic!(ErrorKind::Other, "UDP server unexpectedly terminated");
                    } else {
                        break;
                    }
                }
            };
            *self = next;
        }
        Ok(Async::NotReady)
    }
}
impl<H: HandleMessage> fmt::Debug for UdpServerInner<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UdpServerInner::Binding { .. } => write!(f, "Binding {{ .. }}"),
            UdpServerInner::Running { .. } => write!(f, "Running {{ .. }}"),
        }
    }
}

/// TCP based STUN server.
#[derive(Debug)]
#[must_use = "future do nothing unless polled"]
pub struct TcpServer<S, H>(TcpServerInner<S, H>);
impl<S, H> TcpServer<S, H>
where
    S: Spawn + Clone + Send + 'static,
    H: Factory,
    H::Item: HandleMessage,
{
    /// Starts the server.
    pub fn start(spawner: S, bind_addr: SocketAddr, handler_factory: H) -> Self {
        let inner = TcpServerInner::Binding {
            future: TcpListener::bind(bind_addr),
            spawner: Some(spawner),
            handler_factory: Some(handler_factory),
        };
        TcpServer(inner)
    }
}
impl<S, H> Future for TcpServer<S, H>
where
    S: Spawn + Clone + Send + 'static,
    H: Factory,
    H::Item: HandleMessage + Send + 'static,
    <<H::Item as HandleMessage>::Attribute as Attribute>::Decoder: Send + 'static,
    <<H::Item as HandleMessage>::Attribute as Attribute>::Encoder: Send + 'static,
{
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

// TODO: fibers_transport::TcpListener
enum TcpServerInner<S, H> {
    Binding {
        future: TcpListenerBind,
        spawner: Option<S>,
        handler_factory: Option<H>,
    },
    Listening {
        incoming: Incoming,
        spawner: S,
        handler_factory: H,
    },
}
impl<S, H> Future for TcpServerInner<S, H>
where
    S: Spawn + Clone + Send + 'static,
    H: Factory,
    H::Item: HandleMessage + Send + 'static,
    <<H::Item as HandleMessage>::Attribute as Attribute>::Decoder: Send + 'static,
    <<H::Item as HandleMessage>::Attribute as Attribute>::Encoder: Send + 'static,
{
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next = match self {
                TcpServerInner::Binding {
                    future,
                    spawner,
                    handler_factory,
                } => {
                    if let Async::Ready(listener) = track!(future.poll().map_err(Error::from))? {
                        TcpServerInner::Listening {
                            incoming: listener.incoming(),
                            spawner: spawner.take().expect("never fails"),
                            handler_factory: handler_factory.take().expect("never fails"),
                        }
                    } else {
                        break;
                    }
                }
                TcpServerInner::Listening {
                    incoming,
                    spawner,
                    handler_factory,
                } => {
                    if let Async::Ready(client) = track!(incoming.poll().map_err(Error::from))? {
                        if let Some((future, _)) = client {
                            let boxed_spawner = spawner.clone().boxed();
                            let mut handler = handler_factory.create();
                            let future = future.then(move |result| match result {
                                Err(e) => {
                                    let e = track!(Error::from(e));
                                    handler.handle_channel_error(&e);
                                    Either::A(futures::failed(e))
                                }
                                Ok(stream) => {
                                    let transporter =
                                        match track!(TcpTransporter::from_stream(stream)) {
                                            Ok(t) => t,
                                            Err(e) => {
                                                return Either::A(futures::failed(Error::from(e)))
                                            }
                                        };
                                    let transporter =
                                        StunTcpTransporter::<
                                            _,
                                            TcpTransporter<MessageEncoder<_>, MessageDecoder<_>>,
                                        >::new(transporter);
                                    let channel = Channel::new(transporter);
                                    Either::B(HandlerDriver::new(boxed_spawner, handler, channel))
                                }
                            });
                            spawner.spawn(future.map_err(|_| ()));
                        } else {
                            track_panic!(ErrorKind::Other, "TCP server unexpectedly terminated");
                        }
                    }
                    break;
                }
            };
            *self = next;
        }
        Ok(Async::NotReady)
    }
}
impl<S, H> fmt::Debug for TcpServerInner<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TcpServerInner::Binding { .. } => write!(f, "Binding {{ .. }}"),
            TcpServerInner::Listening { .. } => write!(f, "Listening {{ .. }}"),
        }
    }
}

/// Action instructed by an operation of a message handler.
pub enum Action<T> {
    /// Replies an response to the client immediately.
    Reply(T),

    /// Replies an response to the client in the future.
    FutureReply(Box<Future<Item = T, Error = Never> + Send + 'static>),

    /// Does not reply to the client.
    NoReply,

    /// Does not reply to the client, but does something for handling the incoming message.
    FutureNoReply(Box<Future<Item = (), Error = Never> + Send + 'static>),
}
impl<T: fmt::Debug> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::Reply(t) => write!(f, "Reply({:?})", t),
            Action::FutureReply(_) => write!(f, "FutureReply(_)"),
            Action::NoReply => write!(f, "NoReply"),
            Action::FutureNoReply(_) => write!(f, "FutureNoReply(_)"),
        }
    }
}

/// This trait allows for handling messages sent by clients.
#[allow(unused_variables)]
pub trait HandleMessage {
    /// The attributes that the handler can recognize.
    type Attribute: Attribute + Send + 'static;

    /// Handles a request message.
    ///
    /// The default implementation always returns `Action::NoReply`.
    fn handle_call(
        &mut self,
        peer: SocketAddr,
        request: Request<Self::Attribute>,
    ) -> Action<Response<Self::Attribute>> {
        Action::NoReply
    }

    /// Handles an indication message.
    ///
    /// The default implementation always returns `Action::NoReply`.
    fn handle_cast(
        &mut self,
        peer: SocketAddr,
        indication: Indication<Self::Attribute>,
    ) -> Action<Never> {
        Action::NoReply
    }

    /// Handles an invalid incoming message.
    ///
    /// Note that this method should not return `Action::Reply(_)` or `Action::FutureReply(_)`
    /// if the class of `message` is not `MessageClass::Request`.
    ///
    /// The default implementation always returns `Action::NoReply`.
    fn handle_invalid_message(
        &mut self,
        peer: SocketAddr,
        message: InvalidMessage,
    ) -> Action<Response<Self::Attribute>> {
        Action::NoReply
    }

    /// Handles an error before the channel drops by the error.
    ///
    /// The default implementation does nothing.
    fn handle_channel_error(&mut self, error: &Error) {}
}

#[derive(Debug)]
struct HandlerDriver<H: HandleMessage, T> {
    spawner: BoxSpawn,
    handler: H,
    channel: Channel<H::Attribute, T>,
    response_tx: mpsc::Sender<(SocketAddr, Response<H::Attribute>)>,
    response_rx: mpsc::Receiver<(SocketAddr, Response<H::Attribute>)>,
}
impl<H, T> HandlerDriver<H, T>
where
    H: HandleMessage,
    T: StunTransport<H::Attribute>,
{
    fn new(spawner: BoxSpawn, handler: H, channel: Channel<H::Attribute, T>) -> Self {
        let (response_tx, response_rx) = mpsc::channel();
        HandlerDriver {
            spawner,
            handler,
            channel,
            response_tx,
            response_rx,
        }
    }

    fn handle_message(
        &mut self,
        peer: SocketAddr,
        message: RecvMessage<H::Attribute>,
    ) -> Result<()> {
        match message {
            RecvMessage::Indication(m) => self.handle_indication(peer, m),
            RecvMessage::Request(m) => track!(self.handle_request(peer, m))?,
            RecvMessage::Invalid(m) => track!(self.handle_invalid_message(peer, m))?,
        }
        Ok(())
    }

    fn handle_indication(&mut self, peer: SocketAddr, indication: Indication<H::Attribute>) {
        match self.handler.handle_cast(peer, indication) {
            Action::NoReply => {}
            Action::FutureNoReply(future) => self.spawner.spawn(future.map_err(|_| unreachable!())),
            _ => unreachable!(),
        }
    }

    fn handle_request(&mut self, peer: SocketAddr, request: Request<H::Attribute>) -> Result<()> {
        match self.handler.handle_call(peer, request) {
            Action::NoReply => {}
            Action::FutureNoReply(future) => self.spawner.spawn(future.map_err(|_| unreachable!())),
            Action::Reply(m) => track!(self.channel.reply(peer, m))?,
            Action::FutureReply(future) => {
                let tx = self.response_tx.clone();
                self.spawner.spawn(
                    future
                        .map(move |response| {
                            let _ = tx.send((peer, response));
                            ()
                        }).map_err(|_| unreachable!()),
                );
            }
        }
        Ok(())
    }

    fn handle_invalid_message(&mut self, peer: SocketAddr, message: InvalidMessage) -> Result<()> {
        match self.handler.handle_invalid_message(peer, message) {
            Action::NoReply => {}
            Action::FutureNoReply(future) => self.spawner.spawn(future.map_err(|_| unreachable!())),
            Action::Reply(m) => track!(self.channel.reply(peer, m))?,
            Action::FutureReply(future) => {
                let tx = self.response_tx.clone();
                self.spawner.spawn(
                    future
                        .map(move |response| {
                            let _ = tx.send((peer, response));
                            ()
                        }).map_err(|_| unreachable!()),
                );
            }
        }
        Ok(())
    }
}
impl<H, T> Future for HandlerDriver<H, T>
where
    H: HandleMessage,
    T: StunTransport<H::Attribute>,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut did_something = true;
        while did_something {
            did_something = false;

            match track!(self.channel.poll_recv()) {
                Err(e) => {
                    self.handler.handle_channel_error(&e);
                    return Err(e);
                }
                Ok(Async::NotReady) => {}
                Ok(Async::Ready((peer, message))) => {
                    track!(self.handle_message(peer, message))?;
                    did_something = true;
                }
            }
            if let Err(e) = track!(self.channel.poll_send()) {
                self.handler.handle_channel_error(&e);
                return Err(e);
            }
            if let Async::Ready(item) = self.response_rx.poll().expect("never fails") {
                let (peer, response) = item.expect("never fails");
                track!(self.channel.reply(peer, response))?;
                did_something = true;
            }
        }
        Ok(Async::NotReady)
    }
}

/// Example `BINDING` request handler.
///
/// Note that this is provided only for test and example purposes.
#[derive(Debug, Default, Clone)]
pub struct BindingHandler;
impl HandleMessage for BindingHandler {
    type Attribute = rfc5389::Attribute;

    fn handle_call(
        &mut self,
        peer: SocketAddr,
        request: Request<Self::Attribute>,
    ) -> Action<Response<Self::Attribute>> {
        if request.method() == rfc5389::methods::BINDING {
            let mut response = SuccessResponse::new(&request);
            response.add_attribute(rfc5389::attributes::XorMappedAddress::new(peer).into());
            Action::Reply(Ok(response))
        } else {
            let response = ErrorResponse::new(&request, rfc5389::errors::BadRequest.into());
            Action::Reply(Err(response))
        }
    }
}
