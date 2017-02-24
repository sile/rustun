use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use std::marker::PhantomData;
use fibers::Spawn;
use fibers::sync::mpsc;
use fibers::sync::oneshot::{Monitor, Monitored};
use futures::{self, Future, Poll, Async, BoxFuture};
use futures::future::{Either, Failed};

use {Client, Transport, Method, Attribute, Error, Result};
use message::{Request, Indication, Response, RawMessage};
use types::TransactionId;
use constants;

#[derive(Debug)]
enum Command {
    Cast(RawMessage),
    Call(RawMessage, Monitored<RawMessage, Error>),
}

#[derive(Debug)]
pub struct BaseClient<T> {
    server: SocketAddr,
    request_timeout: Duration,
    command_tx: mpsc::Sender<Command>,
    _phantom: PhantomData<T>,
}
impl<T> BaseClient<T>
    where T: Transport + Send + 'static,
          T::RecvMessage: Send + 'static
{
    pub fn new<S: Spawn>(spawner: &S, server: SocketAddr, transport: T) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        spawner.spawn(BaseClientLoop::new(server, transport, command_rx));
        BaseClient {
            server: server,
            command_tx: command_tx,
            request_timeout: Duration::from_millis(constants::DEFAULT_TI_MS),
            _phantom: PhantomData,
        }
    }
    pub fn set_request_timeout(&mut self, timeout: Duration) {
        self.request_timeout = timeout;
    }
}
impl<T: Transport, M: Method, A: Attribute> Client<M, A> for BaseClient<T> {
    type Call = BaseCall<T, M, A>;
    type Cast = BaseCast<T>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call {
        BaseCall::new(self, message)
    }
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast {
        BaseCast::new(self, message)
    }
}

struct BaseClientLoop<T: Transport> {
    server: SocketAddr,
    transport: T,
    command_rx: mpsc::Receiver<Command>,
    recv_message: T::RecvMessage,
}
impl<T: Transport> BaseClientLoop<T> {
    fn new(server: SocketAddr, mut transport: T, command_rx: mpsc::Receiver<Command>) -> Self {
        let recv_message = transport.recv_message();
        BaseClientLoop {
            server: server,
            transport: transport,
            command_rx: command_rx,
            recv_message: recv_message,
        }
    }
}
impl<T: Transport> Future for BaseClientLoop<T> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        panic!()
    }
}

pub struct BaseCall<T: Transport, M, A>(T, M, A);
impl<T: Transport, M: Method, A: Attribute> BaseCall<T, M, A> {
    fn new(client: &mut BaseClient<T>, message: Request<M, A>) -> Self {
        panic!()
    }
}
impl<T: Transport, M, A> Future for BaseCall<T, M, A> {
    type Item = Response<M, A>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        panic!()
    }
}

pub struct BaseCast<T: Transport>(T);
impl<T: Transport> BaseCast<T> {
    fn new<M: Method, A: Attribute>(client: &mut BaseClient<T>, message: Indication<M, A>) -> Self {
        panic!()
    }
}
impl<T: Transport> Future for BaseCast<T> {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        panic!()
    }
}
