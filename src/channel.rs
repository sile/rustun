use fibers::sync::oneshot;
use futures::{Async, Future, Poll, Stream};
use std;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use stun_codec::{Attribute, BrokenMessage, Message, MessageClass, TransactionId};
use trackable::error::ErrorKindExt;

use constants;
use message::{ErrorResponse, Indication, InvalidMessage, Request, Response, SuccessResponse};
use timeout_queue::TimeoutQueue;
use transport::StunTransport;
use {Error, ErrorKind};

type Reply<A> = oneshot::Monitored<Response<A>, Error>;

#[derive(Debug, Clone)]
pub struct ChannelBuilder {
    request_timeout: Duration,
}
impl ChannelBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_timeout(&mut self, duration: Duration) -> &mut Self {
        self.request_timeout = duration;
        self
    }

    pub fn finish<A, T>(&self, transporter: T) -> Channel<A, T>
    where
        A: Attribute,
        T: StunTransport<A>,
    {
        Channel {
            transporter,
            timeout_queue: TimeoutQueue::new(),
            request_timeout: self.request_timeout,
            transactions: HashMap::new(),
        }
    }
}
impl Default for ChannelBuilder {
    fn default() -> Self {
        ChannelBuilder {
            request_timeout: Duration::from_millis(constants::DEFAULT_TIMEOUT_MS),
        }
    }
}

#[derive(Debug)]
pub struct Channel<A, T> {
    transporter: T,
    timeout_queue: TimeoutQueue<(SocketAddr, TransactionId)>,
    request_timeout: Duration,
    transactions: HashMap<(SocketAddr, TransactionId), Reply<A>>,
}
impl<A, T> Channel<A, T>
where
    A: Attribute,
    T: StunTransport<A>,
{
    pub fn new(transporter: T) -> Self {
        ChannelBuilder::default().finish(transporter)
    }

    pub fn call(
        &mut self,
        peer: SocketAddr,
        request: Request<A>,
    ) -> impl Future<Item = Response<A>, Error = Error> {
        let id = request.transaction_id();
        let (tx, rx) = oneshot::monitor();
        if self.transactions.contains_key(&(peer, id)) {
            let e = ErrorKind::InvalidInput.cause(format!(
                "Transaction ID conflicted: peer={:?}, transaction_id={:?}",
                peer, id
            ));
            tx.exit(Err(track!(e).into()));
        } else {
            self.transactions.insert((peer, id), tx);
            self.timeout_queue.push((peer, id), self.request_timeout);
            self.transporter.send(peer, request.into_message());
        }
        rx.map_err(Error::from)
    }

    pub fn cast(&mut self, peer: SocketAddr, indication: Indication<A>) {
        self.transporter.send(peer, indication.into_message());
    }

    pub fn reply(&mut self, peer: SocketAddr, response: Response<A>) {
        let message = response
            .map(|m| m.into_message())
            .unwrap_or_else(|m| m.into_message());
        self.transporter.send(peer, message);
    }

    pub fn transporter_ref(&self) -> &T {
        &self.transporter
    }

    pub fn transporter_mut(&mut self) -> &mut T {
        &mut self.transporter
    }

    fn handle_timeout(&mut self) {
        let transactions = &mut self.transactions;
        while let Some((peer, id)) = self
            .timeout_queue
            .pop_expired(|entry| transactions.contains_key(entry))
        {
            if let Some(tx) = transactions.remove(&(peer, id)) {
                let e = track!(ErrorKind::Timeout.error());
                tx.exit(Err(e.into()));
            }
            self.transporter.finish_transaction(peer, id);
        }
    }

    fn handle_message(
        &mut self,
        peer: SocketAddr,
        message: std::result::Result<Message<A>, BrokenMessage>,
    ) -> Option<(SocketAddr, RecvMessage<A>)> {
        let message = match message {
            Err(broken) => Some(self.handle_broken_message(broken)),
            Ok(message) => match message.class() {
                MessageClass::Indication => Some(self.handle_indication(message)),
                MessageClass::Request => Some(self.handle_request(message)),
                MessageClass::SuccessResponse => self.handle_success_response(peer, message),
                MessageClass::ErrorResponse => self.handle_error_response(peer, message),
            },
        };
        message.map(|m| (peer, m))
    }

    fn handle_broken_message(&self, message: BrokenMessage) -> RecvMessage<A> {
        RecvMessage::Invalid(InvalidMessage {
            method: message.method(),
            class: message.class(),
            transaction_id: message.transaction_id(),
            error: track!(Error::from(message.error().clone())),
        })
    }

    fn handle_indication(&self, message: Message<A>) -> RecvMessage<A> {
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        match track!(Indication::from_message(message)) {
            Err(error) => RecvMessage::Invalid(InvalidMessage {
                method,
                class,
                transaction_id,
                error,
            }),
            Ok(indication) => RecvMessage::Indication(indication),
        }
    }

    fn handle_request(&self, message: Message<A>) -> RecvMessage<A> {
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        match track!(Request::from_message(message)) {
            Err(error) => RecvMessage::Invalid(InvalidMessage {
                method,
                class,
                transaction_id,
                error,
            }),
            Ok(request) => RecvMessage::Request(request),
        }
    }

    fn handle_success_response(
        &mut self,
        peer: SocketAddr,
        message: Message<A>,
    ) -> Option<RecvMessage<A>> {
        // TODO: check method
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        if let Some(tx) = self.transactions.remove(&(peer, transaction_id)) {
            self.transporter.finish_transaction(peer, transaction_id);
            let result = track!(SuccessResponse::from_message(message)).map(Ok);
            tx.exit(result);
            None
        } else {
            let error = track!(ErrorKind::UnknownTransaction.error()).into();
            let message = RecvMessage::Invalid(InvalidMessage {
                method,
                class,
                transaction_id,
                error,
            });
            Some(message)
        }
    }

    fn handle_error_response(
        &mut self,
        peer: SocketAddr,
        message: Message<A>,
    ) -> Option<RecvMessage<A>> {
        // TODO: check method
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        if let Some(tx) = self.transactions.remove(&(peer, transaction_id)) {
            self.transporter.finish_transaction(peer, transaction_id);
            let result = track!(ErrorResponse::from_message(message)).map(Err);
            tx.exit(result);
            None
        } else {
            let error = track!(ErrorKind::UnknownTransaction.error()).into();
            let message = RecvMessage::Invalid(InvalidMessage {
                method,
                class,
                transaction_id,
                error,
            });
            Some(message)
        }
    }
}
impl<A, T> Stream for Channel<A, T>
where
    A: Attribute,
    T: StunTransport<A>,
{
    type Item = (SocketAddr, RecvMessage<A>);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.handle_timeout();
        while let Some((peer, message)) = self.transporter.recv() {
            if let Some(item) = self.handle_message(peer, message) {
                return Ok(Async::Ready(Some(item)));
            }
        }
        if track!(self.transporter.run_once())? {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

#[derive(Debug)]
pub enum RecvMessage<A> {
    Request(Request<A>),
    Indication(Indication<A>),
    Invalid(InvalidMessage),
}
