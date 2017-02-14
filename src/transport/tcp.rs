use std::time::Duration;
use std::net::SocketAddr;
use fibers::net::TcpStream;
use fibers::time::timer;
use futures::{self, Future, BoxFuture};
use byteorder::{ByteOrder, BigEndian};
use handy_async::pattern::{Pattern, Window};
use handy_async::io::{ReadFrom, WriteInto};

use Error;
use message::RawMessage;
use constants;
use super::{RecvMessage, SendMessage};

#[derive(Debug)]
pub struct TcpSender {
    stream: TcpStream,
    request_timeout: Duration,
}
impl TcpSender {
    pub fn new(stream: TcpStream) -> Self {
        TcpSender {
            stream: stream,
            request_timeout: Duration::from_millis(constants::DEFAULT_TI_MS),
        }
    }
    pub fn set_request_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.request_timeout = timeout;
        self
    }
    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
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
    fn send_request(&mut self, message: RawMessage) -> Self::Future {
        let timeout = timer::timeout(self.request_timeout)
            .map_err(Error::failed)
            .and_then(|_| Err(Error::Timeout));
        let future = self.send_message(message);
        future.select(timeout).map_err(|(e, _)| e).and_then(|(_, next)| next).boxed()
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
