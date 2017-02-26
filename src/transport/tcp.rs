use std::io;
use std::fmt;
use std::net::SocketAddr;
use std::collections::VecDeque;
use fibers::net::TcpStream;
use fibers::sync::oneshot::Link;
use futures::{Future, BoxFuture, Async, Poll, Stream, Sink, AsyncSink, StartSend};
use handy_async::io::{ReadFrom, AsyncWrite};
use handy_async::io::futures::WriteAll;
use handy_async::sync_io::ReadExt;
use handy_async::pattern::{Pattern, Endian, Window};
use handy_async::pattern::read::U16;

use {Result, Error, ErrorKind};
use message::{Class, RawMessage};
use super::{MessageStream, MessageSink, MessageSinkItem, Transport};

/// TCP based implementation of [Transport](trait.Transport.html) trait.
#[derive(Debug)]
pub struct TcpTransport {
    sink: TcpMessageSink,
    stream: TcpMessageStream,
}
impl TcpTransport {
    /// Makes a new `TcpTransport` instance.
    pub fn new(peer: SocketAddr, stream: TcpStream) -> Self {
        TcpTransport {
            sink: TcpMessageSink::new(peer, stream.clone()),
            stream: TcpMessageStream::new(peer, stream),
        }
    }
}
impl Transport for TcpTransport {}
impl MessageSink for TcpTransport {}
impl MessageStream for TcpTransport {}
impl Sink for TcpTransport {
    type SinkItem = MessageSinkItem;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sink.start_send(item)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.poll_complete()
    }
}
impl Stream for TcpTransport {
    type Item = (SocketAddr, Result<RawMessage>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.stream.poll()
    }
}

type MessageSinkState = Option<::std::result::Result<TcpStream, WriteAll<TcpStream, Vec<u8>>>>;

#[derive(Debug)]
struct TcpMessageSink {
    peer: SocketAddr,
    state: MessageSinkState,
    queue: VecDeque<(RawMessage, Option<Link<(), Error, (), ()>>)>,
}
impl TcpMessageSink {
    fn new(peer: SocketAddr, stream: TcpStream) -> Self {
        TcpMessageSink {
            peer: peer,
            state: Some(Ok(stream)),
            queue: VecDeque::new(),
        }
    }
    fn poll_complete_impl(&mut self) -> Poll<(), Error> {
        let mut state = self.state.take().expect("unreachable");
        loop {
            match state {
                Err(mut future) => {
                    let polled = track_try!(future.poll().map_err(|e| e.into_error()));
                    if let Async::Ready((socket, _)) = polled {
                        let (_, link) = self.queue.pop_front().unwrap();
                        if let Some(link) = link {
                            link.exit(Ok(()));
                        }
                        state = Ok(socket);
                    } else {
                        self.state = Some(Err(future));
                        return Ok(Async::NotReady);
                    }
                }
                Ok(stream) => {
                    if let Some(&(ref message, _)) = self.queue.front() {
                        state = Err(stream.async_write_all(message.to_bytes()));
                    } else {
                        return Ok(Async::Ready(()));
                    }
                }
            }
        }
    }
}
impl Sink for TcpMessageSink {
    type SinkItem = MessageSinkItem;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let (addr, message, link) = item;
        track_assert_eq!(addr, self.peer, ErrorKind::Other);
        self.queue.push_back((message, link));
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match track_err!(self.poll_complete_impl()) {
            Err(e) => {
                for link in self.queue.drain(..).filter_map(|(_, link)| link) {
                    link.exit(Err(e.clone()));
                }
                Err(e)
            }
            Ok(v) => Ok(v),

        }
    }
}
impl MessageSink for TcpMessageSink {}

type RecvMessageBytes = BoxFuture<(TcpStream, Vec<u8>), io::Error>;

struct TcpMessageStream {
    peer: SocketAddr,
    future: RecvMessageBytes,
}
impl TcpMessageStream {
    fn new(peer: SocketAddr, stream: TcpStream) -> Self {
        TcpMessageStream {
            peer: peer,
            future: Self::recv_message_bytes(stream),
        }
    }
    fn recv_message_bytes(stream: TcpStream) -> RecvMessageBytes {
        let pattern = vec![0; 20]
            .and_then(|mut buf| {
                let message_len = (&mut &buf[2..4]).read_u16be().unwrap();
                buf.resize(20 + message_len as usize, 0);
                Window::new(buf).skip(20)
            })
            .and_then(|buf| buf.into_inner());
        pattern.read_from(stream).map_err(|e| e.into_error()).boxed()
    }
}
impl Stream for TcpMessageStream {
    type Item = (SocketAddr, Result<RawMessage>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let polled = track_try!(match self.future.poll() {
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Ok(Async::Ready(None));
                }
                Err(e)
            }
            Ok(v) => Ok(v),
        });
        if let Async::Ready((stream, bytes)) = polled {
            let message = track_err!(RawMessage::read_from(&mut &bytes[..]));
            self.future = Self::recv_message_bytes(stream);
            Ok(Async::Ready(Some((self.peer, message))))
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl MessageStream for TcpMessageStream {}
impl fmt::Debug for TcpMessageStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TcpMessageStream {{ peer: {:?}, future: _ }}", self.peer)
    }
}
