use std::net::SocketAddr;
use fibers::sync::oneshot::Link;
use futures::{Sink, Stream};

use {Result, Error};
use message::RawMessage;

pub use self::udp2::{UdpTransportBuilder, UdpTransport};

pub mod futures {
    pub use super::udp2::UdpTransportBind;
}

// mod udp;
mod udp2;

pub type MessageSinkItem = (SocketAddr, RawMessage, Link<(), (), (), Error>);

pub trait MessageSink: Sink<SinkItem = MessageSinkItem, SinkError = Error> {}
pub trait MessageStream
    : Stream<Item = (SocketAddr, Result<RawMessage>), Error = Error> {
}
pub trait Transport: MessageSink + MessageStream {}
