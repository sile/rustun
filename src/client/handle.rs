use fibers::sync::mpsc;
use futures::{Async, Future, Poll, Stream};
use stun_codec::{Attribute, Method};

use super::Client;
use message::{Indication, Request, Response};
use {AsyncReply, AsyncResult, Error};

#[derive(Debug, Clone)]
pub struct ClientHandle<M, A> {
    tx: mpsc::Sender<Command<M, A>>,
}
impl<M: Method, A: Attribute> ClientHandle<M, A> {
    pub(crate) fn new<C: Client<M, A>>(client: C) -> (Self, ClientDriver<M, A, C>) {
        let (tx, rx) = mpsc::channel();
        (
            ClientHandle { tx },
            ClientDriver {
                client,
                rx,
                last_error: None,
            },
        )
    }

    pub fn call(&self, request: Request<M, A>) -> AsyncResult<Response<M, A>> {
        let (reply, response) = AsyncResult::new();
        let _ = self.tx.send(Command::Call(request, reply));
        response
    }

    pub fn cast(&self, indication: Indication<M, A>) {
        let _ = self.tx.send(Command::Cast(indication));
    }
}

#[derive(Debug)]
pub struct ClientDriver<M, A, C> {
    client: C,
    rx: mpsc::Receiver<Command<M, A>>,
    last_error: Option<Error>,
}
impl<M, A, C> ClientDriver<M, A, C>
where
    M: Method,
    A: Attribute,
    C: Client<M, A>,
{
    fn handle_command(&mut self, command: Command<M, A>) {
        match command {
            Command::Call(request, reply) => {
                if let Some(e) = self.last_error.clone() {
                    reply.send(Err(track!(e)));
                } else {
                    self.client.call_with_reply(request, reply);
                }
            }
            Command::Cast(indication) => {
                if self.last_error.is_none() {
                    self.client.cast(indication);
                }
            }
        }
    }
}
impl<M, A, C> Future for ClientDriver<M, A, C>
where
    M: Method,
    A: Attribute,
    C: Client<M, A>,
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(command) = self.rx.poll().expect("never fails") {
            if let Some(command) = command {
                self.handle_command(command);
            } else {
                // All handles have been dropped
                return Ok(Async::Ready(()));
            }
        }
        if self.last_error.is_none() {
            if let Err(e) = track!(self.client.poll()) {
                self.last_error = Some(e);
            }
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
enum Command<M, A> {
    Call(Request<M, A>, AsyncReply<Response<M, A>>),
    Cast(Indication<M, A>),
}
