//! Basic STUN servers.
//!
//! This module provides only a basic STUN servers.
//! If you want more elaborate one, please consider create your own server using [`Channel`] directly.
//!
//! [`Channel`]: ../channel/struct.Channel.html
use crate::channel::{Channel, RecvMessage};
use crate::message::{
    ErrorResponse, Indication, InvalidMessage, Request, Response, SuccessResponse,
};
use crate::transport::{StunTcpTransporter, StunTransport, StunUdpTransporter};
use crate::{Error, ErrorKind, Result};
use bytecodec::marker::Never;
use factory::DefaultFactory;
use factory::Factory;
use fibers::sync::mpsc;
use fibers::{BoxSpawn, Spawn};
use fibers_transport::{self, FixedPeerTransporter, TcpTransport, UdpTransport};
use futures::{Async, Future, Poll, Stream};
use std::fmt;
use std::net::SocketAddr;
use stun_codec::rfc5389;
use stun_codec::{Attribute, MessageDecoder, MessageEncoder};

/// The default TCP and UDP port for STUN.
pub const DEFAULT_PORT: u16 = 3478;

/// The default TLS port for STUN.
pub const DEFAULT_TLS_PORT: u16 = 5349;

type UdpTransporter<A> = fibers_transport::UdpTransporter<MessageEncoder<A>, MessageDecoder<A>>;

/// UDP based STUN server.
#[derive(Debug)]
#[must_use = "future do nothing unless polled"]
pub struct UdpServer<H: HandleMessage> {
    driver: HandlerDriver<H, StunUdpTransporter<H::Attribute, UdpTransporter<H::Attribute>>>,
}
impl<H: HandleMessage> UdpServer<H> {
    /// Starts the server.
    pub fn start<S>(
        spawner: S,
        bind_addr: SocketAddr,
        handler: H,
    ) -> impl Future<Item = Self, Error = Error>
    where
        S: Spawn + Send + 'static,
    {
        UdpTransporter::bind(bind_addr)
            .map_err(|e| track!(Error::from(e)))
            .map(move |transporter| {
                let channel = Channel::new(StunUdpTransporter::new(transporter));
                let driver = HandlerDriver::new(spawner.boxed(), handler, channel, true);
                UdpServer { driver }
            })
    }

    /// Returns the address to which the server is bound.
    pub fn local_addr(&self) -> SocketAddr {
        self.driver
            .channel
            .transporter_ref()
            .inner_ref()
            .local_addr()
    }
}
impl<H: HandleMessage> Future for UdpServer<H> {
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(()) = track!(self.driver.poll())? {
            track_panic!(ErrorKind::Other, "STUN UDP server unexpectedly terminated");
        }
        Ok(Async::NotReady)
    }
}

type TcpListener<A> = fibers_transport::TcpListener<
    DefaultFactory<MessageEncoder<A>>,
    DefaultFactory<MessageDecoder<A>>,
>;

/// TCP based STUN server.
#[must_use = "future do nothing unless polled"]
pub struct TcpServer<S, H>
where
    H: Factory,
    H::Item: HandleMessage,
{
    spawner: S,
    handler_factory: H,
    listener: TcpListener<<H::Item as HandleMessage>::Attribute>,
}
impl<S, H> TcpServer<S, H>
where
    S: Spawn + Clone + Send + 'static,
    H: Factory,
    H::Item: HandleMessage,
{
    /// Starts the server.
    pub fn start(
        spawner: S,
        bind_addr: SocketAddr,
        handler_factory: H,
    ) -> impl Future<Item = Self, Error = Error> {
        TcpListener::listen(bind_addr)
            .map_err(|e| track!(Error::from(e)))
            .map(move |listener| TcpServer {
                spawner,
                handler_factory,
                listener,
            })
    }

    /// Returns the address to which the server is bound.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr()
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
        while let Async::Ready(transporter) = track!(self.listener.poll())? {
            if let Some(transporter) = transporter {
                let peer_addr = transporter.peer_addr();
                let transporter =
                    FixedPeerTransporter::new(peer_addr, (), StunTcpTransporter::new(transporter));
                let channel = Channel::new(transporter);
                let handler = self.handler_factory.create();
                let future =
                    HandlerDriver::new(self.spawner.clone().boxed(), handler, channel, false);
                self.spawner.spawn(future.map_err(|_| ()));
            } else {
                track_panic!(ErrorKind::Other, "STUN TCP server unexpectedly terminated");
            }
        }
        Ok(Async::NotReady)
    }
}
impl<S, H> fmt::Debug for TcpServer<S, H>
where
    H: Factory,
    H::Item: HandleMessage,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TcpServer {{ .. }}")
    }
}

/// Action instructed by an operation of a message handler.
pub enum Action<T> {
    /// Replies an response to the client immediately.
    Reply(T),

    /// Replies an response to the client in the future.
    FutureReply(Box<dyn Future<Item = T, Error = Never> + Send + 'static>),

    /// Does not reply to the client.
    NoReply,

    /// Does not reply to the client, but does something for handling the incoming message.
    FutureNoReply(Box<dyn Future<Item = (), Error = Never> + Send + 'static>),
}
impl<T: fmt::Debug> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::Reply(t) => write!(f, "Reply({t:?})"),
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
struct HandlerDriver<H, T>
where
    H: HandleMessage,
    T: StunTransport<H::Attribute, PeerAddr = SocketAddr>,
{
    spawner: BoxSpawn,
    handler: H,
    channel: Channel<H::Attribute, T>,
    response_tx: mpsc::Sender<(SocketAddr, Response<H::Attribute>)>,
    response_rx: mpsc::Receiver<(SocketAddr, Response<H::Attribute>)>,
    recoverable_channel: bool,
}
impl<H, T> HandlerDriver<H, T>
where
    H: HandleMessage,
    T: StunTransport<H::Attribute, PeerAddr = SocketAddr>,
{
    fn new(
        spawner: BoxSpawn,
        handler: H,
        channel: Channel<H::Attribute, T>,
        recoverable_channel: bool,
    ) -> Self {
        let (response_tx, response_rx) = mpsc::channel();
        HandlerDriver {
            spawner,
            handler,
            channel,
            response_tx,
            response_rx,
            recoverable_channel,
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
                        })
                        .map_err(|_| unreachable!()),
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
                        })
                        .map_err(|_| unreachable!()),
                );
            }
        }
        Ok(())
    }
}
impl<H, T> Future for HandlerDriver<H, T>
where
    H: HandleMessage,
    T: StunTransport<H::Attribute, PeerAddr = SocketAddr>,
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
                    if !self.recoverable_channel {
                        return Err(e);
                    }
                    did_something = true;
                }
                Ok(Async::NotReady) => {}
                Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                Ok(Async::Ready(Some((peer, message)))) => {
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

    fn handle_channel_error(&mut self, error: &Error) {
        eprintln!("[ERROR] {error}");
    }
}
