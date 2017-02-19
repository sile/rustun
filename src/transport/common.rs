use std::net::SocketAddr;
use futures::{Future, Stream, Poll, Async};

use Error;
use message::RawMessage;
use super::RecvMessage;

pub fn message_stream<T: RecvMessage>(t: T) -> MessageStream<T> {
    MessageStream(t.recv_message())
}

pub struct MessageStream<T: RecvMessage>(T::Future);
impl<T: RecvMessage> Stream for MessageStream<T> {
    type Item = (SocketAddr, RawMessage);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Async::Ready((receiver, addr, message)) = track_err!(self.0.poll())? {
            self.0 = receiver.recv_message();
            Ok(Async::Ready(Some((addr, message))))
        } else {
            Ok(Async::NotReady)
        }
    }
}
