use std::net::SocketAddr;
use futures::Future;

pub use self::tcp::{TcpSender, TcpReceiver};
pub use self::udp::{UdpSender, UdpReceiver, UdpRetransmissionSpec};

use Error;
use message::RawMessage;

pub mod futures {
    pub use super::tcp::{TcpRecvMessage, TcpSendMessage};
    pub use super::udp::{UdpRecvMessage, UdpSendMessage};
}

pub mod streams {
    pub use super::common::MessageStream;
}

mod tcp;
mod udp;
mod common;

pub trait SendMessage {
    type Future: Future<Item = (), Error = Error>;
    fn send_message(&mut self, message: RawMessage) -> Self::Future;
    fn send_request(&mut self, message: RawMessage) -> Self::Future;
}

pub trait RecvMessage: Sized {
    type Future: Future<Item = (Self, SocketAddr, RawMessage), Error = Error>;
    fn recv_message(self) -> Self::Future;
    fn into_stream(self) -> streams::MessageStream<Self> {
        common::message_stream(self)
    }
}
