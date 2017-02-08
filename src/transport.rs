use std::io::{Read, Write};
use futures::{Sink, Stream};

use {Error, StunMethod, Attribute};
use message::Message;

pub trait Transport: Read + Write {}

pub trait TransportChannel<M, A>
    where M: StunMethod,
          A: Attribute
{
    type Sender: Sink<SinkItem = Message<M, A>, SinkError = Error>;
    type Receiver: Stream<Item = Message<M, A>, Error = Error>;
    fn channel(self) -> (Self::Sender, Self::Receiver);
}
