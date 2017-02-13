use futures::{Sink, Stream};

use Error;
use msg::RawMessage;

pub mod tcp_channel;

pub trait Channel {
    type Sender: Sink<SinkItem = RawMessage, SinkError = Error>;
    type Receiver: Stream<Item = Result<RawMessage, Error>, Error = Error>;
    fn channel(self) -> (Self::Sender, Self::Receiver);
}
