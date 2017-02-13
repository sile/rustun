use std::net::SocketAddr;
use fibers::net::TcpStream;
use futures::{self, Future, BoxFuture};
use byteorder::{ByteOrder, BigEndian};
use handy_async::pattern::{Pattern, Window};
use handy_async::io::{ReadFrom, WriteInto};

use Error;
use message::RawMessage;
use super::{RecvMessage, SendMessage};

#[derive(Debug)]
pub struct TcpSender {
    stream: TcpStream,
}
impl TcpSender {
    pub fn new(stream: TcpStream) -> Self {
        TcpSender { stream: stream }
    }
}
impl SendMessage for TcpSender {
    type Future = TcpSendMessage;
    fn send_message(&mut self, message: RawMessage) -> Self::Future {
        let mut buf = Vec::new();
        let result = may_fail!(message.write_to(&mut buf));
        let stream = self.stream.clone();
        futures::done(result)
            .and_then(move |_| {
                let future = buf.write_into(stream)
                    .map(|_| ())
                    .map_err(|e| Error::failed(e.into_error()));
                may_fail!(future)
            })
            .boxed()
    }
}

#[derive(Debug)]
pub struct TcpReceiver(TcpStream);
impl TcpReceiver {
    pub fn new(stream: TcpStream) -> Self {
        TcpReceiver(stream)
    }
}
impl RecvMessage for TcpReceiver {
    type Future = TcpRecvMessage;
    fn recv_message(self) -> Self::Future {
        let pattern = vec![0; 20].and_then(|mut buf| {
            let attrs_len = BigEndian::read_u16(&buf[2..4]) as usize;
            buf.resize(20 + attrs_len, 0);
            Window::new(buf).set_start(20)
        });
        may_fail!(pattern.read_from(self.0).map_err(|e| Error::failed(e.into_error())))
            .and_then(|(stream, buf)| {
                let peer = may_fail!(stream.peer_addr().map_err(Error::from))?;
                let message = may_fail!(RawMessage::read_from(&mut &buf.into_inner()[..]))?;
                Ok((TcpReceiver(stream), peer, message))
            })
            .boxed()
    }
}

pub type TcpSendMessage = BoxFuture<(), Error>;
pub type TcpRecvMessage = BoxFuture<(TcpReceiver, SocketAddr, RawMessage), Error>;
