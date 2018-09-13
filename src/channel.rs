//! Channel for sending and receiving STUN messages.
use fibers::sync::oneshot;
use fibers_timeout_queue::TimeoutQueue;
use futures::{Async, Future, Poll, Stream};
use std;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use stun_codec::{Attribute, BrokenMessage, Message, MessageClass, Method, TransactionId};
use trackable::error::ErrorKindExt;

use message::{
    ErrorResponse, Indication, InvalidMessage, MessageError, MessageErrorKind, Request, Response,
    SuccessResponse,
};
use transport::StunTransport;
use Error;

type Reply<A> = oneshot::Monitored<Response<A>, MessageError>;

/// [`Channel`] builder.
///
/// [`Channel`]: ./struct.Channel.html
#[derive(Debug, Clone)]
#[must_use = "streams do nothing unless polled"]
pub struct ChannelBuilder {
    request_timeout: Duration,
}
impl ChannelBuilder {
    /// The default value of `request_timeout`.
    ///
    /// > Reliability of STUN over TCP and TLS-over-TCP is handled by TCP
    /// > itself, and there are no retransmissions at the STUN protocol level.
    /// > However, for a request/response transaction, if the client has not
    /// > received a response by **Ti** seconds after it sent the SYN to establish
    /// > the connection, it considers the transaction to have timed out.  **Ti**
    /// > SHOULD be configurable and SHOULD have a default of **39.5s**.
    /// >
    /// > [RFC 5389 -- 7.2.2. Sending over TCP or TLS-over-TCP]
    ///
    /// [RFC 5389 -- 7.2.2. Sending over TCP or TLS-over-TCP]: https://tools.ietf.org/html/rfc5389#section-7.2.2
    pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 39_500;

    /// Makes a new `ChannelBuilder` instance with the default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the request timeout duration of the channel.
    ///
    /// The default value is `DEFAULT_REQUEST_TIMEOUT_MS`.
    pub fn request_timeout(&mut self, duration: Duration) -> &mut Self {
        self.request_timeout = duration;
        self
    }

    /// Makes a new `Channel` instance with the given settings.
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
            request_timeout: Duration::from_millis(Self::DEFAULT_REQUEST_TIMEOUT_MS),
        }
    }
}

/// Channel for sending and receiving STUN messages.
#[derive(Debug)]
pub struct Channel<A, T> {
    transporter: T,
    timeout_queue: TimeoutQueue<(SocketAddr, TransactionId)>,
    request_timeout: Duration,
    transactions: HashMap<(SocketAddr, TransactionId), (Method, Reply<A>)>,
}
impl<A, T> Channel<A, T>
where
    A: Attribute,
    T: StunTransport<A>,
{
    /// Makes a new `Channel` instance.
    ///
    /// This is equivalent to `ChannelBuilder::default().finish(transporter)`.
    pub fn new(transporter: T) -> Self {
        ChannelBuilder::default().finish(transporter)
    }

    /// Sends the given request message to the destination peer and
    /// returns a future that waits the corresponding response.
    #[cfg_attr(feature = "cargo-clippy", allow(map_entry))]
    pub fn call(
        &mut self,
        peer: SocketAddr,
        request: Request<A>,
    ) -> impl Future<Item = Response<A>, Error = MessageError> {
        let id = request.transaction_id();
        let (tx, rx) = oneshot::monitor();
        if self.transactions.contains_key(&(peer, id)) {
            let e = MessageErrorKind::InvalidInput.cause(format!(
                "Transaction ID conflicts: peer={:?}, transaction_id={:?}",
                peer, id
            ));
            tx.exit(Err(track!(e).into()));
        } else {
            self.transactions.insert((peer, id), (request.method(), tx));
            self.timeout_queue.push((peer, id), self.request_timeout);
            self.transporter.send(peer, request.into_message());
        }
        rx.map_err(MessageError::from)
    }

    /// Sends the given indication message to the destination peer.
    pub fn cast(&mut self, peer: SocketAddr, indication: Indication<A>) {
        self.transporter.send(peer, indication.into_message());
    }

    /// Replies the given response message to the destination peer.
    pub fn reply(&mut self, peer: SocketAddr, response: Response<A>) {
        let message = response
            .map(|m| m.into_message())
            .unwrap_or_else(|m| m.into_message());
        self.transporter.send(peer, message);
    }

    /// Returns a reference to the transporter of the channel.
    pub fn transporter_ref(&self) -> &T {
        &self.transporter
    }

    /// Returns a mutable reference to the transporter of the channel.
    pub fn transporter_mut(&mut self) -> &mut T {
        &mut self.transporter
    }

    /// Returns the number of the outstanding request/response transactions in the channel.
    pub fn outstanding_transactions(&self) -> usize {
        self.transactions.len()
    }

    fn handle_timeout(&mut self) {
        let transactions = &mut self.transactions;
        while let Some((peer, id)) = self
            .timeout_queue
            .filter_pop(|entry| transactions.contains_key(entry))
        {
            if let Some((_, tx)) = transactions.remove(&(peer, id)) {
                let e = track!(MessageErrorKind::Timeout.error());
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
            Err(broken) => Some(self.handle_broken_message(&broken)),
            Ok(message) => match message.class() {
                MessageClass::Indication => Some(self.handle_indication(message)),
                MessageClass::Request => Some(self.handle_request(message)),
                MessageClass::SuccessResponse => self.handle_success_response(peer, message),
                MessageClass::ErrorResponse => self.handle_error_response(peer, message),
            },
        };
        message.map(|m| (peer, m))
    }

    fn handle_broken_message(&self, message: &BrokenMessage) -> RecvMessage<A> {
        let bytecodec_error_kind = *message.error().kind();
        let error = MessageErrorKind::MalformedAttribute.takes_over(message.error().clone());
        RecvMessage::Invalid(InvalidMessage::new(
            message.method(),
            message.class(),
            message.transaction_id(),
            track!(error; bytecodec_error_kind).into(),
        ))
    }

    fn handle_indication(&self, message: Message<A>) -> RecvMessage<A> {
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        match track!(Indication::from_message(message)) {
            Err(error) => {
                RecvMessage::Invalid(InvalidMessage::new(method, class, transaction_id, error))
            }
            Ok(indication) => RecvMessage::Indication(indication),
        }
    }

    fn handle_request(&self, message: Message<A>) -> RecvMessage<A> {
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        match track!(Request::from_message(message)) {
            Err(error) => {
                RecvMessage::Invalid(InvalidMessage::new(method, class, transaction_id, error))
            }
            Ok(request) => RecvMessage::Request(request),
        }
    }

    fn handle_success_response(
        &mut self,
        peer: SocketAddr,
        message: Message<A>,
    ) -> Option<RecvMessage<A>> {
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        if let Some((method, tx)) = self.transactions.remove(&(peer, transaction_id)) {
            self.transporter.finish_transaction(peer, transaction_id);
            let result = track!(SuccessResponse::from_message(message))
                .and_then(|m| {
                    track_assert_eq!(m.method(), method, MessageErrorKind::UnexpectedResponse);
                    Ok(m)
                })
                .map(Ok);
            tx.exit(result);
            None
        } else {
            let error =
                track!(MessageErrorKind::UnexpectedResponse.cause("Unknown transaction ID")).into();
            let message =
                RecvMessage::Invalid(InvalidMessage::new(method, class, transaction_id, error));
            Some(message)
        }
    }

    fn handle_error_response(
        &mut self,
        peer: SocketAddr,
        message: Message<A>,
    ) -> Option<RecvMessage<A>> {
        let class = message.class();
        let method = message.method();
        let transaction_id = message.transaction_id();
        if let Some((method, tx)) = self.transactions.remove(&(peer, transaction_id)) {
            self.transporter.finish_transaction(peer, transaction_id);
            let result = track!(ErrorResponse::from_message(message))
                .and_then(|m| {
                    track_assert_eq!(m.method(), method, MessageErrorKind::UnexpectedResponse);
                    Ok(m)
                })
                .map(Err);
            tx.exit(result);
            None
        } else {
            let error =
                track!(MessageErrorKind::UnexpectedResponse.cause("Unknown transaction ID")).into();
            let message =
                RecvMessage::Invalid(InvalidMessage::new(method, class, transaction_id, error));
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

        match track!(self.transporter.run_once()) {
            Err(e) => {
                let message_error = track!(MessageError::from(e.clone()));
                for (_, (_, reply)) in self.transactions.drain() {
                    reply.exit(Err(message_error.clone()));
                }
                Err(e)
            }
            Ok(true) => {
                let e = MessageError::from(track!(
                    MessageErrorKind::Other.cause("Transporter terminated")
                ));
                for (_, (_, reply)) in self.transactions.drain() {
                    reply.exit(Err(e.clone()));
                }
                Ok(Async::Ready(None))
            }
            Ok(false) => Ok(Async::NotReady),
        }
    }
}

/// Received message.
///
/// Messages are received by calling `Channel::poll` method.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum RecvMessage<A> {
    Request(Request<A>),
    Indication(Indication<A>),
    Invalid(InvalidMessage),
}
