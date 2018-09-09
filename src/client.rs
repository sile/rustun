use fibers::sync::{mpsc, oneshot};
use fibers::Spawn;
use futures::stream::Fuse;
use futures::{Async, Future, IntoFuture, Poll, Stream};
use std::net::SocketAddr;
use stun_codec::Attribute;

use channel::Channel;
use message::{Indication, Request, Response};
use transport::StunTransport;
use {Error, Result};

#[derive(Debug, Clone)]
pub struct Client<A> {
    command_tx: mpsc::Sender<Command<A>>,
}
impl<A> Client<A> {
    pub fn new<S, T>(spawner: S, channel: Channel<A, T>) -> Self
    where
        S: Spawn + Clone + Send + 'static,
        A: Attribute + Send + 'static,
        T: StunTransport<A> + Send + 'static,
    {
        let (command_tx, command_rx) = mpsc::channel();
        let channel_driver = ChannelDriver {
            spawner: spawner.clone(),
            channel: Ok(channel),
            command_rx: command_rx.fuse(),
        };
        spawner.spawn(channel_driver);
        Client { command_tx }
    }

    pub fn call(
        &self,
        peer: SocketAddr,
        request: Request<A>,
    ) -> impl Future<Item = Response<A>, Error = Error> {
        let (tx, rx) = oneshot::monitor();
        let command = Command::Call(peer, request, tx);
        track!(self.command_tx.send(command).map_err(Error::from))
            .into_future()
            .and_then(move |()| rx.map_err(|e| track!(Error::from(e))))
    }

    pub fn cast(&self, peer: SocketAddr, indication: Indication<A>) -> Result<()> {
        let command = Command::Cast(peer, indication);
        track!(self.command_tx.send(command).map_err(Error::from))
    }
}

#[derive(Debug)]
enum Command<A> {
    Call(
        SocketAddr,
        Request<A>,
        oneshot::Monitored<Response<A>, Error>,
    ),
    Cast(SocketAddr, Indication<A>),
}

#[derive(Debug)]
struct ChannelDriver<S, A, T> {
    spawner: S,
    channel: Result<Channel<A, T>>,
    command_rx: Fuse<mpsc::Receiver<Command<A>>>,
}
impl<S, A, T> ChannelDriver<S, A, T>
where
    S: Spawn,
    A: Attribute + Send + 'static,
    T: StunTransport<A> + Send + 'static,
{
    fn handle_command(&mut self, command: Command<A>) {
        match command {
            Command::Cast(peer, indication) => {
                if let Ok(channel) = self.channel.as_mut() {
                    channel.cast(peer, indication);
                }
            }
            Command::Call(peer, request, reply) => match self.channel {
                Err(ref e) => {
                    reply.exit(Err(track!(e.clone())));
                }
                Ok(ref mut channel) => {
                    let future = channel.call(peer, request).map_err(Error::from).then(
                        move |result| {
                            reply.exit(track!(result));
                            Ok(())
                        },
                    );
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
            match track!(self.channel.as_mut().expect("never fails").poll()) {
                Err(e) => {
                    self.channel = Err(e);
                }
                Ok(Async::NotReady) => {
                    break;
                }
                Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                Ok(Async::Ready(Some(_message))) => {
                    // All received messages are ignored
                }
            }
        }
        Ok(Async::NotReady)
    }
}
