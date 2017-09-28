use std::net::SocketAddr;
use fibers::{Spawn, BoxSpawn};
use fibers::sync::mpsc;
use futures::{Future, Poll, Async, Stream, AsyncSink};
use trackable::error::ErrorKindExt;

use {Result, Error, HandleMessage, ErrorKind};
use message::{Class, RawMessage};
use transport::Transport;
use super::IndicationSender;

/// Base STUN server that can be used as base of other implementations.
#[derive(Debug)]
pub struct BaseServer;
impl BaseServer {
    /// Starts the STUN server.
    pub fn start<S, T, H>(spawner: S, transport: T, mut handler: H) -> BaseServerLoop<T, H>
    where
        S: Spawn + Send + 'static,
        T: Transport,
        H: HandleMessage,
    {
        let (info_tx, info_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();
        handler.on_init(info_tx.clone(), IndicationSender::new(response_tx.clone()));
        BaseServerLoop {
            spawner: spawner.boxed(),
            transport: transport,
            handler: handler,
            _info_tx: info_tx,
            info_rx: info_rx,
            response_tx: response_tx,
            response_rx: response_rx,
        }
    }
}

/// `Future` that represent the loop of a UDP server for handling transactions issued by clients.
pub struct BaseServerLoop<T, H: HandleMessage> {
    spawner: BoxSpawn,
    transport: T,
    handler: H,
    _info_tx: mpsc::Sender<H::Info>,
    info_rx: mpsc::Receiver<H::Info>,
    response_tx: mpsc::Sender<(SocketAddr, Result<RawMessage>)>,
    response_rx: mpsc::Receiver<(SocketAddr, Result<RawMessage>)>,
}
impl<T, H> BaseServerLoop<T, H>
where
    T: Transport,
    H: HandleMessage,
    H::HandleCall: Send + 'static,
    H::HandleCast: Send + 'static,
{
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
                let e = ErrorKind::Invalid.cause(format!("Unexpected class: {:?}", other));
                Err(track!(e))
            }
        }
    }
}
impl<T, H> Future for BaseServerLoop<T, H>
where
    T: Transport,
    H: HandleMessage,
    H::HandleCall: Send + 'static,
    H::HandleCast: Send + 'static,
{
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut do_something = false;

            match track_try!(self.info_rx.poll().map_err(|()| ErrorKind::Other)) {
                Async::NotReady => {}
                Async::Ready(None) => unreachable!(),
                Async::Ready(Some(info)) => {
                    do_something = true;
                    self.handler.handle_info(info);
                }
            }

            track_try!(self.transport.poll_complete());
            match track_try!(self.response_rx.poll().map_err(|()| ErrorKind::Other)) {
                Async::NotReady => {}
                Async::Ready(None) => unreachable!(),
                Async::Ready(Some((client, Err(error)))) => {
                    do_something = true;
                    self.handler.handle_error(client, error);
                }
                Async::Ready(Some((client, Ok(message)))) => {
                    do_something = true;
                    let started = track_try!(self.transport.start_send((client, message, None)));
                    if let AsyncSink::NotReady((client, message, _)) = started {
                        let e = track!(
                            ErrorKind::Full.error(),
                            "Cannot response to transaction {:?}",
                            message.transaction_id()
                        );
                        self.handler.handle_error(client, e);
                    }
                }
            }

            match track_try!(self.transport.poll()) {
                Async::NotReady => {}
                Async::Ready(None) => return Ok(Async::Ready(())),
                Async::Ready(Some((client, message))) => {
                    do_something = true;
                    if let Err(e) = message.and_then(|m| self.handle_message(client, m)) {
                        self.handler.handle_error(client, e);
                    }
                }
            }
            if !do_something {
                return Ok(Async::NotReady);
            }
        }
    }
}
