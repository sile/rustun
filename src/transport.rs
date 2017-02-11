use std::io::{Read, Write};
use std::net::SocketAddr;
use std::marker::PhantomData;
use futures::{Sink, Stream, Poll, Async, StartSend, Future, AsyncSink};
use fibers::net::UdpSocket;
use fibers::net::futures::{RecvFrom, SendTo};
use failure::Failure;

use DEFAULT_MAX_MESSAGE_SIZE;
use {Error, StunMethod, Attribute};
use message::Message;

pub trait Transport: Read + Write {}

pub trait TransportChannel<M, A>
    where M: StunMethod,
          A: Attribute
{
    type Sender: Sink<SinkItem = Message<M, A>, SinkError = Error>;
    type Receiver: Stream<Item = Result<Message<M, A>, Error>, Error = Error>;
    fn channel(self) -> (Self::Sender, Self::Receiver);
}

// TODO: built-in
#[derive(Debug)]
pub struct UdpChannel {
    socket: UdpSocket,
    peer: SocketAddr,
}
impl UdpChannel {
    pub fn new(socket: UdpSocket, peer: SocketAddr) -> Self {
        UdpChannel {
            socket: socket,
            peer: peer,
        }
    }
}
impl<M, A> TransportChannel<M, A> for UdpChannel
    where M: StunMethod,
          A: Attribute
{
    type Sender = UdpSender<M, A>;
    type Receiver = UdpReceiver<M, A>;
    fn channel(self) -> (Self::Sender, Self::Receiver) {
        let tx = UdpSender::new(self.socket.clone(), self.peer);
        let rx = UdpReceiver::new(self.socket);
        (tx, rx)
    }
}

#[derive(Debug)]
enum UdpSenderInner {
    Idle(UdpSocket, SocketAddr),
    Busy(SendTo<Vec<u8>>, SocketAddr),
    Polled,
}
impl UdpSenderInner {
    pub fn take(&mut self) -> UdpSenderInner {
        use std::mem;
        mem::replace(self, UdpSenderInner::Polled)
    }
}

// NOTE: unreliable channel (no retry)

#[derive(Debug)]
pub struct UdpSender<M, A> {
    inner: UdpSenderInner,
    _phantom: PhantomData<(M, A)>,
}
impl<M, A> UdpSender<M, A> {
    fn new(socket: UdpSocket, peer: SocketAddr) -> Self {
        UdpSender {
            inner: UdpSenderInner::Idle(socket, peer),
            _phantom: PhantomData,
        }
    }
}
impl<M, A> Sink for UdpSender<M, A>
    where M: StunMethod,
          A: Attribute
{
    type SinkItem = Message<M, A>;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        if let UdpSenderInner::Busy(..) = self.inner {
            Ok(AsyncSink::NotReady(item))
        } else if let UdpSenderInner::Idle(socket, peer) = self.inner.take() {
            let bytes = item.try_into_bytes()?;
            self.inner = UdpSenderInner::Busy(socket.send_to(bytes, peer), peer);
            Ok(AsyncSink::Ready)
        } else {
            unreachable!()
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.inner.take() {
            inner @ UdpSenderInner::Idle(..) => {
                self.inner = inner;
                Ok(Async::Ready(()))
            }
            UdpSenderInner::Busy(mut f, peer) => {
                if let Async::Ready((socket, _, _)) =
                    may_fail!(f.poll().map_err(|(_, _, e)| Failure::new(e)))? {
                    self.inner = UdpSenderInner::Idle(socket, peer);
                    Ok(Async::Ready(()))
                } else {
                    self.inner = UdpSenderInner::Busy(f, peer);
                    Ok(Async::NotReady)
                }
            }
            UdpSenderInner::Polled => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct UdpReceiver<M, A> {
    future: RecvFrom<Vec<u8>>,
    _phantom: PhantomData<(M, A)>,
}
impl<M, A> UdpReceiver<M, A> {
    fn new(socket: UdpSocket) -> Self {
        UdpReceiver {
            future: socket.recv_from(vec![0; DEFAULT_MAX_MESSAGE_SIZE]),
            _phantom: PhantomData,
        }
    }
}
impl<M, A> Stream for UdpReceiver<M, A>
    where M: StunMethod,
          A: Attribute
{
    type Item = Result<Message<M, A>, Error>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Async::Ready((socket, buf, size, _)) =
            may_fail!(self.future.poll().map_err(|(_, _, e)| Failure::new(e)))? {
            let result = Message::try_from_bytes(&buf[..size]);
            self.future = socket.recv_from(buf);
            Ok(Async::Ready(Some(result)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
