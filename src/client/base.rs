use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use std::marker::PhantomData;
use fibers::Spawn;
use fibers::sync::mpsc;
use fibers::sync::oneshot::{self, Monitor, Monitored, Link};
use fibers::time::timer::{self, Timeout};
use futures::{Future, Stream, Poll, Async, AsyncSink};
use trackable::error::ErrorKindExt;

use {Client, Transport, Error, ErrorKind, Result};
use message::RawMessage;
use types::TransactionId;
use constants;

#[derive(Debug)]
enum Command {
    Cast(RawMessage, Link<(), Error, (), ()>),
    Call(RawMessage, Link<(), Error, (), ()>, Monitored<RawMessage, Error>),
    Abort(TransactionId),
}

/// A [Client](trait.Client.html) trait implementation which
/// can be used as base of other implementations.
///
/// This uses `T` as the message transport layer.
#[derive(Debug)]
pub struct BaseClient<T> {
    server: SocketAddr,
    request_timeout: Duration,
    command_tx: mpsc::Sender<Command>,
    _phantom: PhantomData<T>,
}
impl<T> BaseClient<T>
    where T: Transport + Send + 'static
{
    /// Makes a new `BaseClient` instance which communicates with `server`.
    pub fn new<S: Spawn>(spawner: &S, server: SocketAddr, transport: T) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        spawner.spawn(BaseClientLoop::new(server, transport, command_rx));
        BaseClient {
            server: server,
            command_tx: command_tx,
            request_timeout: Duration::from_millis(constants::DEFAULT_TIMEOUT_MS),
            _phantom: PhantomData,
        }
    }

    /// Sets the timeout duration of a request transaction.
    ///
    /// The default value is [DEFAULT_TIMEOUT_MS](../constants/constant.DEFAULT_TIMEOUT_MS.html).
    pub fn set_request_timeout(&mut self, timeout: Duration) {
        self.request_timeout = timeout;
    }
}
impl<T: Transport> Client for BaseClient<T> {
    type CallRaw = BaseCallRaw;
    type CastRaw = BaseCastRaw;
    fn call_raw(&mut self, message: RawMessage) -> Self::CallRaw {
        BaseCallRaw::new(self, message)
    }
    fn cast_raw(&mut self, message: RawMessage) -> Self::CastRaw {
        BaseCastRaw::new(self, message)
    }
}

struct BaseClientLoop<T: Transport> {
    server: SocketAddr,
    transport: T,
    command_rx: mpsc::Receiver<Command>,
    transactions: HashMap<TransactionId, Monitored<RawMessage, Error>>,
}
impl<T: Transport> BaseClientLoop<T> {
    fn new(server: SocketAddr, transport: T, command_rx: mpsc::Receiver<Command>) -> Self {
        BaseClientLoop {
            server: server,
            transport: transport,
            command_rx: command_rx,
            transactions: HashMap::new(),
        }
    }
    fn handle_message(&mut self, _peer: SocketAddr, message: RawMessage) {
        if let Some(monitored) = self.transactions.remove(message.transaction_id()) {
            monitored.exit(Ok(message));
        }
    }
    fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Cast(message, link) => {
                let result = track_try!(self.transport
                    .start_send((self.server, message, Some(link))));
                if let AsyncSink::NotReady((_, _, Some(link))) = result {
                    link.exit(track_err!(Err(ErrorKind::Full)));
                }
            }
            Command::Call(message, link, monitored) => {
                let transaction_id = message.transaction_id().clone();
                let result = track_try!(self.transport
                    .start_send((self.server, message, Some(link))));
                if let AsyncSink::NotReady((_, _, Some(link))) = result {
                    link.exit(track_err!(Err(ErrorKind::Full)));
                } else {
                    self.transactions.insert(transaction_id, monitored);
                }
            }
            Command::Abort(transaction_id) => {
                self.transactions.remove(&transaction_id);
            }
        }
        Ok(())
    }
    fn handle_error(&mut self, error: Error) {
        for (_, monitored) in self.transactions.drain() {
            monitored.exit(Err(error.clone()));
        }
    }
}
impl<T: Transport> Future for BaseClientLoop<T> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let result = (|| loop {
            let no_transaction = track_try!(self.transport.poll_complete()).is_ready();
            if !no_transaction {
                match track_try!(self.transport.poll()) {
                    Async::NotReady => {}
                    Async::Ready(None) => return track_err!(Err(disconnected())),
                    Async::Ready(Some((peer, _local, message))) => {
                        // NOTE: Invalid messages are silently discard.
                        if let Ok(message) = message {
                            self.handle_message(peer, message);
                        }
                    }
                }
            }
            match track_try!(self.command_rx.poll().map_err(|()| ErrorKind::Other)) {
                Async::NotReady => return Ok(Async::NotReady),
                Async::Ready(None) => return Ok(Async::Ready(())),
                Async::Ready(Some(command)) => {
                    track_try!(self.handle_command(command));
                }
            }
        })();
        result.map_err(|e| {
            self.handle_error(e);
            ()
        })
    }
}

/// `Future` that handle a request/response transaction issued by `BaseClient`.
pub struct BaseCallRaw {
    transaction_id: TransactionId,
    _link: Link<(), (), (), Error>,
    monitor: Monitor<RawMessage, Error>,
    timeout: Timeout,
    command_tx: Option<mpsc::Sender<Command>>,
}
impl BaseCallRaw {
    fn new<T>(client: &mut BaseClient<T>, message: RawMessage) -> Self {
        let transaction_id = message.transaction_id().clone();
        let (link0, link1) = oneshot::link();
        let (monitored, monitor) = oneshot::monitor();
        let _ = client.command_tx.send(Command::Call(message, link1, monitored));
        BaseCallRaw {
            transaction_id: transaction_id,
            _link: link0,
            monitor: monitor,
            timeout: timer::timeout(client.request_timeout),
            command_tx: Some(client.command_tx.clone()),
        }
    }
}
impl Future for BaseCallRaw {
    type Item = RawMessage;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(message) = track_try!(self.monitor.poll()) {
            self.command_tx = None;
            return Ok(Async::Ready(message));
        }
        if let Async::Ready(()) = track_try!(self.timeout.poll()) {
            return Err(track!(ErrorKind::Timeout.error()));
        }
        Ok(Async::NotReady)
    }
}
impl Drop for BaseCallRaw {
    fn drop(&mut self) {
        if let Some(command_tx) = self.command_tx.take() {
            let _ = command_tx.send(Command::Abort(self.transaction_id));
        }
    }
}

/// `Future` that handle a indication transaction issued by `BaseClient`.
pub struct BaseCastRaw {
    link: Link<(), (), (), Error>,
    // NOTE: This field is kept to maintain reference count of the receiver side.
    _command_tx: mpsc::Sender<Command>,
}
impl BaseCastRaw {
    fn new<T>(client: &mut BaseClient<T>, message: RawMessage) -> Self {
        let (link0, link1) = oneshot::link();
        let _ = client.command_tx.send(Command::Cast(message, link1));
        BaseCastRaw {
            link: link0,
            _command_tx: client.command_tx.clone(),
        }
    }
}
impl Future for BaseCastRaw {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        track_err!(self.link.poll())
    }
}

fn disconnected() -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionAborted, "Disconnected")
}
