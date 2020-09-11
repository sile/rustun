//! Basic STUN client.
//!
//! This module provides only a basic STUN client.
//! If you want more elaborate one, please consider create your own client using [`Channel`] directly.
//!
//! [`Channel`]: ../channel/struct.Channel.html
use crate::channel::Channel;
use crate::message::{Indication, Request, Response};
use crate::transport::StunTransport;
use crate::{Error, Result};
use fibers::sync::{mpsc, oneshot};
use fibers::Spawn;
use futures::stream::Fuse;
use futures::{Async, Future, IntoFuture, Poll, Stream};
use std::fmt;
use std::marker::PhantomData;
use stun_codec::Attribute;

/// STUN client.
#[derive(Debug, Clone)]
pub struct Client<A, T>
where
    A: Attribute,
    T: StunTransport<A>,
{
    command_tx: mpsc::Sender<Command<A, T::PeerAddr>>,
    _phantom: PhantomData<T>,
}
impl<A, T> Client<A, T>
where
    A: Attribute + Send + 'static,
    T: StunTransport<A> + Send + 'static,
    T::PeerAddr: Send + 'static,
{
    /// Makes a new `Client` instance that uses the given channel for sending/receiving messages.
    pub fn new<S>(spawner: &S, channel: Channel<A, T>) -> Self
    where
        S: Spawn + Clone + Send + 'static,
    {
        let (command_tx, command_rx) = mpsc::channel();
        let channel_driver = ChannelDriver {
            spawner: spawner.clone(),
            channel: Ok(channel),
            command_rx: command_rx.fuse(),
        };
        spawner.spawn(channel_driver);
        Client {
            command_tx,
            _phantom: PhantomData,
        }
    }

    /// Sends the given request message to the destination peer and
    /// returns a future that waits the corresponding response.
    pub fn call(
        &self,
        peer: T::PeerAddr,
        request: Request<A>,
    ) -> impl Future<Item = Response<A>, Error = Error> {
        let (tx, rx) = oneshot::monitor();
        let command = Command::Call(peer, request, tx);
        track!(self.command_tx.send(command).map_err(Error::from))
            .into_future()
            .and_then(move |()| rx.map_err(|e| track!(Error::from(e))))
    }

    /// Sends the given indication message to the destination peer.
    ///
    /// # Errors
    ///
    /// If the channel being used by the client has dropped,
    /// this will return an `ErrorKind::Other` error.
    pub fn cast(&self, peer: T::PeerAddr, indication: Indication<A>) -> Result<()> {
        let command = Command::Cast(peer, indication);
        track!(self.command_tx.send(command).map_err(Error::from))
    }
}

enum Command<A, P> {
    Call(P, Request<A>, oneshot::Monitored<Response<A>, Error>),
    Cast(P, Indication<A>),
}
impl<A, P> fmt::Debug for Command<A, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::Call(..) => write!(f, "Call(..)"),
            Command::Cast(..) => write!(f, "Cast(..)"),
        }
    }
}

struct ChannelDriver<S, A, T>
where
    A: Attribute,
    T: StunTransport<A>,
{
    spawner: S,
    channel: Result<Channel<A, T>>,
    command_rx: Fuse<mpsc::Receiver<Command<A, T::PeerAddr>>>,
}
impl<S, A, T> ChannelDriver<S, A, T>
where
    S: Spawn,
    A: Attribute + Send + 'static,
    T: StunTransport<A> + Send + 'static,
{
    fn handle_command(&mut self, command: Command<A, T::PeerAddr>) {
        match command {
            Command::Cast(peer, indication) => {
                if let Ok(channel) = self.channel.as_mut() {
                    let _ = channel.cast(peer, indication);
                }
            }
            Command::Call(peer, request, reply) => match self.channel {
                Err(ref e) => {
                    reply.exit(Err(track!(e.clone())));
                }
                Ok(ref mut channel) => {
                    let future =
                        channel
                            .call(peer, request)
                            .map_err(Error::from)
                            .then(move |result| {
                                reply.exit(track!(result));
                                Ok(())
                            });
                    self.spawner.spawn(future);
                }
            },
        }
    }
}
impl<S, A, T> Future for ChannelDriver<S, A, T>
where
    S: Spawn,
    A: Attribute + Send + 'static,
    T: StunTransport<A> + Send + 'static,
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(command) = self.command_rx.poll().expect("never fails") {
            if let Some(command) = command {
                self.handle_command(command);
            } else {
                // All clients have dropped
                let outstanding_transactions = self
                    .channel
                    .as_mut()
                    .ok()
                    .map_or(0, |c| c.outstanding_transactions());
                if outstanding_transactions == 0 {
                    return Ok(Async::Ready(()));
                } else {
                    break;
                }
            }
        }

        while self.channel.is_ok() {
            match track!(self.channel.as_mut().expect("never fails").poll_recv()) {
                Err(e) => {
                    self.channel = Err(e);
                    break;
                }
                Ok(Async::NotReady) => {}
                Ok(Async::Ready(_message)) => {
                    // All received messages are ignored
                    continue;
                }
            }
            if let Err(e) = track!(self.channel.as_mut().expect("never fails").poll_send()) {
                self.channel = Err(e);
            }
            break;
        }
        Ok(Async::NotReady)
    }
}
