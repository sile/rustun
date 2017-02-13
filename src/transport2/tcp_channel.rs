use fibers::net::TcpStream;
use futures::{Sink, Stream, Poll, StartSend, BoxFuture, Future, Async, AsyncSink};
use handy_async::pattern::{Pattern, Window};
use handy_async::io::{ReadFrom, WriteInto};

use {Result, Error};
use msg::RawMessage;

use super::Channel;

#[derive(Debug)]
pub struct TcpChannel {
    stream: TcpStream,
}
impl Channel for TcpChannel {
    type Sender = TcpSender;
    type Receiver = TcpReceiver;
    fn channel(self) -> (Self::Sender, Self::Receiver) {
        let tx = TcpSender(Some(Ok(self.stream.clone())));
        let rx = TcpReceiver(TcpReceiver::recv_message(self.stream));
        (tx, rx)
    }
}

pub struct TcpSender(Option<::std::result::Result<TcpStream, SendMessage>>);
impl TcpSender {
    fn send_message(stream: TcpStream, message: &RawMessage) -> Result<SendMessage> {
        let mut buf = Vec::new();
        may_fail!(message.write_to(&mut buf))?;
        let future =
            buf.write_into(stream).map(|(s, _)| s).map_err(|e| Error::failed(e.into_error()));
        Ok(future.boxed())
    }
}
impl Sink for TcpSender {
    type SinkItem = RawMessage;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match self.0.take().unwrap() {
            Err(future) => {
                self.0 = Some(Err(future));
                Ok(AsyncSink::NotReady(item))
            }
            Ok(stream) => {
                let future = TcpSender::send_message(stream, &item)?;
                self.0 = Some(Err(future));
                Ok(AsyncSink::Ready)
            }
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.0.take().unwrap() {
            Err(mut future) => {
                if let Async::Ready(stream) = future.poll()? {
                    self.0 = Some(Ok(stream));
                    Ok(Async::Ready(()))
                } else {
                    self.0 = Some(Err(future));
                    Ok(Async::NotReady)
                }
            }
            Ok(stream) => {
                self.0 = Some(Ok(stream));
                Ok(Async::Ready(()))
            }
        }
    }
}

pub struct TcpReceiver(RecvMessage);
impl TcpReceiver {
    fn recv_message(stream: TcpStream) -> RecvMessage {
        use byteorder::{ByteOrder, BigEndian};
        let pattern = vec![0; 20]
            .and_then(|mut buf| {
                let attrs_len = BigEndian::read_u16(&buf[2..4]) as usize;
                buf.resize(20 + attrs_len, 0);
                Window::new(buf).set_start(20)
            })
            .and_then(|buf| Ok(RawMessage::read_from(&mut &buf.into_inner()[..])));
        pattern.read_from(stream)
            .map_err(|e| Error::failed(e.into_error()))
            .boxed()
    }
}
impl Stream for TcpReceiver {
    type Item = Result<RawMessage>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Async::Ready((stream, message)) = may_fail!(self.0.poll())? {
            self.0 = TcpReceiver::recv_message(stream);
            Ok(Async::Ready(Some(message)))
        } else {
            Ok(Async::NotReady)
        }
    }
}

type SendMessage = BoxFuture<TcpStream, Error>;
type RecvMessage = BoxFuture<(TcpStream, Result<RawMessage>), Error>;
