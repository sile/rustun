use futures::{Async, Poll, Stream};
use std::collections::HashMap;
use std::mem;
use std::net::SocketAddr;
use std::time::Duration;
use stun_codec::{Attribute, Message, MessageClass, Method, TransactionId};
use trackable::error::ErrorKindExt;

use super::timeout_queue::TimeoutQueue;
use super::Client;
use constants;
use message::{ErrorResponse, Indication, Request, Response, SuccessResponse};
use transport::{StunTransport, TcpTransport};
use {AsyncReply, AsyncResult, Error, ErrorKind, Result};

// TODO: TcpClientBuidler

#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
pub struct TcpClient<M, A, T> {
    transporter: T,
    transactions: HashMap<TransactionId, (M, AsyncReply<Response<M, A>>)>,
    request_timeout: Duration,
    timeout_queue: TimeoutQueue<TransactionId>,
}
impl<M, A, T> TcpClient<M, A, T>
where
    M: Method,
    A: Attribute,
    T: StunTransport<M, A> + TcpTransport,
{
    pub fn new(transporter: T) -> Self {
        TcpClient {
            transporter,
            transactions: HashMap::new(),
            request_timeout: Duration::from_millis(constants::DEFAULT_TIMEOUT_MS),
            timeout_queue: TimeoutQueue::new(),
        }
    }

    /// Sets the timeout duration of a request transaction.
    ///
    /// The default value is [DEFAULT_TIMEOUT_MS](../constants/constant.DEFAULT_TIMEOUT_MS.html).
    pub fn set_request_timeout(&mut self, timeout: Duration) {
        self.request_timeout = timeout;
    }

    fn handle_message(&mut self, message: Message<M, A>) -> Option<Indication<M, A>> {
        if message.class() == MessageClass::Indication {
            return Some(Indication::from_message(message).expect("never fails"));
        }

        if let Some((request_method, reply)) = self.transactions.remove(message.transaction_id()) {
            reply.send(track!(self.make_response(request_method, message)));
        }
        None
    }

    fn handle_timeout(&mut self, transaction_id: TransactionId) {
        if let Some((_, reply)) = self.transactions.remove(&transaction_id) {
            let e = track!(ErrorKind::Timeout.error());
            reply.send(Err(e.into()));
        }
    }

    fn make_response(&self, request_method: M, response: Message<M, A>) -> Result<Response<M, A>> {
        track_assert_eq!(
            request_method.as_u12(),
            response.method().as_u12(),
            ErrorKind::InvalidInput
        );

        match response.class() {
            MessageClass::SuccessResponse => {
                track!(SuccessResponse::from_message(response)).map(Ok)
            }
            MessageClass::ErrorResponse => track!(ErrorResponse::from_message(response)).map(Err),
            class => {
                track_panic!(
                    ErrorKind::InvalidInput,
                    "Unexpected class of response message: {:?}",
                    class
                );
            }
        }
    }

    fn poll_expired(&mut self) -> Option<TransactionId> {
        let transactions = &self.transactions;
        self.timeout_queue
            .pop_expired(|id| transactions.contains_key(id))
    }
}
impl<M, A, T> Client<M, A> for TcpClient<M, A, T>
where
    M: Method,
    A: Attribute,
    T: StunTransport<M, A> + TcpTransport,
{
    fn call(&mut self, request: Request<M, A>) -> AsyncResult<Response<M, A>> {
        let unused: SocketAddr = unsafe { mem::zeroed() };
        let (tx, rx) = AsyncResult::new();
        self.transactions.insert(
            request.transaction_id().clone(),
            (request.method().clone(), tx),
        );
        self.timeout_queue
            .push(request.transaction_id().clone(), self.request_timeout);
        self.transporter.send(unused, request.into_message());
        rx
    }

    fn cast(&mut self, indication: Indication<M, A>) {
        let unused: SocketAddr = unsafe { mem::zeroed() };
        self.transporter.send(unused, indication.into_message());
    }
}
impl<M, A, T> Stream for TcpClient<M, A, T>
where
    M: Method,
    A: Attribute,
    T: StunTransport<M, A> + TcpTransport,
{
    type Item = Indication<M, A>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        while let Some((_, message)) = self.transporter.recv() {
            if let Some(indication) = self.handle_message(message) {
                return Ok(Async::Ready(Some(indication)));
            }
        }

        while let Some(id) = self.poll_expired() {
            self.handle_timeout(id);
        }

        if track!(self.transporter.poll_finish())? {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}
