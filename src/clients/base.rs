use std::net::SocketAddr;
use std::marker::PhantomData;
use std::collections::HashMap;
use std::sync::Arc;
use fibers::Spawn;
use fibers::sync::oneshot::{self, Link, Monitor, Monitored};
use fibers::sync::mpsc;
use futures::{Future, Poll, Async, Stream};

use {Client, Method, Attribute, Message, Error, Result};
use transport::{SendMessage, RecvMessage};
use transport::streams::MessageStream;
use message::{Indication, Request, Response, RawMessage};
use types::TransactionId;

#[derive(Debug)]
enum Command {
    StartTransaction(TransactionId, Monitored<RawMessage, Error>),
    AbortTransaction(TransactionId),
}

type RecvLoopLink = Arc<Link<(), (), (), Error>>;

#[derive(Debug)]
pub struct BaseClient<S, R> {
    sender: S,
    command_tx: mpsc::Sender<Command>,
    link: RecvLoopLink,
    _phantom: PhantomData<R>,
}
impl<S, R> BaseClient<S, R>
    where S: SendMessage,
          R: RecvMessage + 'static,
          R::Future: Send + 'static
{
    pub fn new<T: Spawn>(spawner: T, sender: S, receiver: R) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let link = spawner.spawn_link(RecvLoop::new(receiver, command_rx));
        BaseClient {
            sender: sender,
            command_tx: command_tx,
            link: Arc::new(link),
            _phantom: PhantomData,
        }
    }
}
impl<M, A, S, R> Client<M, A> for BaseClient<S, R>
    where M: Method,
          A: Attribute,
          S: SendMessage,
          R: RecvMessage
{
    type Call = BaseCall<M, A, S::Future>;
    type Cast = BaseCast<S::Future>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call {
        BaseCall(Some(may_fail!(message.into_inner().try_into_raw()).map(|message| {
            let id = message.transaction_id();
            let future = self.sender.send_request(message);
            BaseCallInner::new(id, future, self.command_tx.clone(), self.link.clone())
        })))
    }
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast {
        BaseCast(Some(may_fail!(message.into_inner().try_into_raw())
            .map(|message| self.sender.send_message(message))))
    }
}

struct RecvLoop<R: RecvMessage> {
    message_rx: MessageStream<R>,
    command_rx: mpsc::Receiver<Command>,
    transactions: HashMap<TransactionId, Monitored<RawMessage, Error>>,
}
impl<R: RecvMessage> RecvLoop<R> {
    fn new(receiver: R, command_rx: mpsc::Receiver<Command>) -> Self {
        RecvLoop {
            message_rx: receiver.into_stream(),
            command_rx: command_rx,
            transactions: HashMap::new(),
        }
    }
    fn handle_command(&mut self, command: Command) {
        match command {
            Command::StartTransaction(id, monitored) => {
                self.transactions.insert(id, monitored);
            }
            Command::AbortTransaction(id) => {
                self.transactions.remove(&id);
            }
        }
    }
    fn handle_message(&mut self, _server: SocketAddr, message: RawMessage) {
        if let Some(monitored) = self.transactions.remove(&message.transaction_id()) {
            monitored.exit(Ok(message));
        }
    }
}
impl<R: RecvMessage> Future for RecvLoop<R> {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(command) =
            may_fail!(self.command_rx.poll().map_err(|_| Error::failed("disconnected")))? {
            let command = command.expect("unreachable");
            self.handle_command(command);
        }
        while let Async::Ready(value) = may_fail!(self.message_rx.poll())? {
            let (addr, message) = value.expect("unreachable");
            self.handle_message(addr, message);
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
pub struct BaseCall<M, A, F>(Option<Result<BaseCallInner<M, A, F>>>);
impl<M, A, F> Future for BaseCall<M, A, F>
    where M: Method,
          A: Attribute,
          F: Future<Item = (), Error = Error>
{
    type Item = Response<M, A>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut f = self.0.take().expect("Cannot poll BaseCall twice")?;
        let result = f.poll()?;
        if let Async::NotReady = result {
            self.0 = Some(Ok(f));
        }
        Ok(result)
    }
}

#[derive(Debug)]
struct BaseCallInner<M, A, F> {
    id: TransactionId,
    send_req: F,
    recv_res: Monitor<RawMessage, Error>,
    command_tx: Option<mpsc::Sender<Command>>,
    link: RecvLoopLink,
    _phantom: PhantomData<(M, A)>,
}
unsafe impl<M: Send, A: Send, F: Send> Send for BaseCallInner<M, A, F> {}
impl<M, A, F> BaseCallInner<M, A, F> {
    fn new(id: TransactionId,
           future: F,
           command_tx: mpsc::Sender<Command>,
           link: RecvLoopLink)
           -> Self {
        let (monitored, monitor) = oneshot::monitor();
        let _ = command_tx.send(Command::StartTransaction(id, monitored));
        BaseCallInner {
            id: id,
            send_req: future,
            recv_res: monitor,
            command_tx: Some(command_tx),
            link: link,
            _phantom: PhantomData,
        }
    }
}
impl<M, A, F> Drop for BaseCallInner<M, A, F> {
    fn drop(&mut self) {
        if let Some(tx) = self.command_tx.take() {
            let _ = tx.send(Command::AbortTransaction(self.id));
        }
    }
}
impl<M, A, F> Future for BaseCallInner<M, A, F>
    where M: Method,
          A: Attribute,
          F: Future<Item = (), Error = Error>
{
    type Item = Response<M, A>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(_) = may_fail!(self.send_req.poll())? {
            // TODO:
            Err(Error::Timeout)
        } else if let Async::Ready(raw) = may_fail!(self.recv_res.poll().map_err(Error::from))? {
            let message = may_fail!(Message::try_from_raw(raw))?;
            let response = may_fail!(message.try_into_response())?;
            self.command_tx = None;
            Ok(Async::Ready(response))
        } else {
            Ok(Async::NotReady)
        }
    }
}

#[derive(Debug)]
pub struct BaseCast<F>(Option<Result<F>>);
impl<F> Future for BaseCast<F>
    where F: Future<Item = (), Error = Error>
{
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut f = self.0.take().expect("Cannot poll BaseCast twice")?;
        let result = f.poll()?;
        if let Async::NotReady = result {
            self.0 = Some(Ok(f));
        }
        Ok(result)
    }
}
