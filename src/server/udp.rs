use std::fmt;
use std::net::SocketAddr;
use fibers::{Spawn, BoxSpawn};
use fibers::sync::mpsc;
use futures::{Future, Poll, Async, Stream, Sink, AsyncSink};
use futures::future::Either;
use trackable::error::ErrorKindExt;

use {Result, Error, HandleMessage, ErrorKind};
use message::{Class, RawMessage};
use transport::{UdpTransport, UdpTransportBuilder};
use transport::futures::UdpTransportBind;

/// UDP STUN server.
#[derive(Debug)]
pub struct UdpServer {
    bind_addr: SocketAddr,
}
impl UdpServer {
    /// Makes a new `UdpServer` instance which will bind to `bind_addr`.
    pub fn new(bind_addr: SocketAddr) -> Self {
        UdpServer { bind_addr: bind_addr }
    }

    /// Starts the UDP server with `handler`.
    pub fn start<S, H>(&self, spawner: S, handler: H) -> UdpServerLoop<H>
        where S: Spawn + Send + 'static,
              H: HandleMessage
    {
        let future = UdpTransportBuilder::new().bind_addr(self.bind_addr).finish();
        let (response_tx, response_rx) = mpsc::channel();
        UdpServerLoop {
            spawner: spawner.boxed(),
            transport: Either::A(future),
            handler: handler,
            response_tx: response_tx,
            response_rx: response_rx,
        }
    }
}

/// `Future` that represent the loop of a UDP server for handling transactions issued by clients.
pub struct UdpServerLoop<H> {
    spawner: BoxSpawn,
    transport: Either<UdpTransportBind, UdpTransport>,
    handler: H,
    response_tx: mpsc::Sender<(SocketAddr, Result<RawMessage>)>,
    response_rx: mpsc::Receiver<(SocketAddr, Result<RawMessage>)>,
}
impl<H: fmt::Debug> fmt::Debug for UdpServerLoop<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "UdpServerLoop {{ spawner: {:?}, transport: {}, handler: {:?}, response_tx: {:?} \
                response_rx: {:?} }}",
               self.spawner,
               match self.transport {
                   Either::A(ref a) => format!("A({:?})", a),
                   Either::B(ref b) => format!("B({:?})", b),
               },
               self.handler,
               self.response_tx,
               self.response_rx)
    }
}
impl<H: HandleMessage> UdpServerLoop<H>
    where H::HandleCall: Send + 'static,
          H::HandleCast: Send + 'static
{
    fn poll_bind_if_needed(&mut self) -> Poll<(), Error> {
        let transport = match self.transport {
            Either::A(ref mut future) => {
                if let Async::Ready(transport) = track_try!(future.poll()) {
                    transport
                } else {
                    return Ok(Async::NotReady);
                }
            }
            Either::B(_) => return Ok(Async::Ready(())),
        };
        self.transport = Either::B(transport);
        Ok(Async::Ready(()))
    }
    fn handle_message(&mut self, client: SocketAddr, message: RawMessage) -> Result<()> {
        match message.class() {
            Class::Request => {
                let request = track_try!(message.try_into_request());
                let future = self.handler.handle_call(client, request);
                let response_tx = self.response_tx.clone();
                let future = future.and_then(move |response| {
                    let message = RawMessage::try_from_response(response);
                    let _ = response_tx.send((client, message));
                    Ok(())
                });
                self.spawner.spawn(future);
                Ok(())
            }
            Class::Indication => {
                let indication = track_try!(message.try_into_indication());
                let future = self.handler.handle_cast(client, indication);
                self.spawner.spawn(future);
                Ok(())
            }
            other => {
                let e = ErrorKind::Other.cause(format!("Unexpected class: {:?}", other));
                Err(track!(e))
            }
        }
    }
}
impl<H: HandleMessage> Future for UdpServerLoop<H>
    where H::HandleCall: Send + 'static,
          H::HandleCast: Send + 'static
{
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::NotReady = track_try!(self.poll_bind_if_needed()) {
            return Ok(Async::NotReady);
        }
        loop {
            let (client, message) = if let Either::B(ref mut transport) = self.transport {
                track_try!(transport.poll_complete());
                match track_try!(self.response_rx.poll().map_err(|()| ErrorKind::Other)) {
                    Async::NotReady => {}
                    Async::Ready(None) => unreachable!(),
                    Async::Ready(Some((client, Err(error)))) => {
                        self.handler.handle_error(client, error);
                    }
                    Async::Ready(Some((client, Ok(message)))) => {
                        let started = track_try!(transport.start_send((client, message, None)));
                        if let AsyncSink::NotReady((client, message, _)) = started {
                            let e = track!(ErrorKind::Full.error(),
                                           "Cannot response to transaction {:?}",
                                           message.transaction_id());
                            self.handler.handle_error(client, e);
                        }
                    }
                }

                match track_try!(transport.poll()) {
                    Async::NotReady => return Ok(Async::NotReady),
                    Async::Ready(None) => return Ok(Async::Ready(())),
                    Async::Ready(Some((client, message))) => (client, message),
                }
            } else {
                unreachable!()
            };
            if let Err(e) = message.and_then(|m| self.handle_message(client, m)) {
                self.handler.handle_error(client, e);
            }
        }
    }
}
