use std::io;
use std::fmt;
use std::net::SocketAddr;
use std::collections::{VecDeque, HashMap};
use std::sync::mpsc::SendError;
use fibers::{Spawn, BoxSpawn};
use fibers::net::{TcpStream, TcpListener};
use fibers::net::futures::{TcpListenerBind, Connected};
use fibers::net::streams::Incoming;
use fibers::sync::mpsc;
use fibers::sync::oneshot::Link;
use futures::{Future, BoxFuture, Async, Poll, Stream, Sink, AsyncSink, StartSend};
use handy_async::io::{ReadFrom, AsyncWrite};
use handy_async::io::futures::WriteAll;
use handy_async::sync_io::ReadExt;
use handy_async::pattern::{Pattern, Window};
use trackable::error::ErrorKindExt;

use {Result, Error, ErrorKind};
use message::RawMessage;
use super::{MessageStream, MessageSink, MessageSinkItem, Transport};

#[derive(Debug)]
enum OutgoingCommand {
    Send(RawMessage, Option<Link<(), Error, (), ()>>),
}

#[derive(Debug)]
enum IncomingCommand {
    Recv(SocketAddr, Result<RawMessage>),
    Exit(SocketAddr, Result<()>),
}

#[derive(Debug)]
enum Listener {
    Bind(TcpListenerBind),
    Incoming(Incoming),
}
impl Stream for Listener {
    type Item = (Connected, SocketAddr);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let next = match *self {
                Listener::Bind(ref mut future) => {
                    if let Async::Ready(listener) = track_try!(future.poll()) {
                        Listener::Incoming(listener.incoming())
                    } else {
                        return Ok(Async::NotReady);
                    }
                }
                Listener::Incoming(ref mut stream) => return track_err!(stream.poll()),
            };
            *self = next;
        }
    }
}

/// TCP based server-side implementation of [Transport](trait.Transport.html) trait.
#[derive(Debug)]
pub struct TcpServerTransport {
    spawner: BoxSpawn,
    listener: Listener,
    clients: HashMap<SocketAddr, mpsc::Sender<OutgoingCommand>>,
    incoming_tx: mpsc::Sender<IncomingCommand>,
    incoming_rx: mpsc::Receiver<IncomingCommand>,
}
impl TcpServerTransport {
    /// Makes a new `TcpServerTransport` instance.
    pub fn new<S>(spawner: S, bind_addr: SocketAddr) -> Self
        where S: Spawn + Send + 'static
    {
        let (incoming_tx, incoming_rx) = mpsc::channel();
        TcpServerTransport {
            spawner: spawner.boxed(),
            listener: Listener::Bind(TcpListener::bind(bind_addr)),
            clients: HashMap::new(),
            incoming_tx: incoming_tx,
            incoming_rx: incoming_rx,
        }
    }
    fn handle_new_connection(&mut self, client: SocketAddr, connected: Connected) {
        let incoming_tx0 = self.incoming_tx.clone();
        let incoming_tx1 = self.incoming_tx.clone();
        let (outgoing_tx, outgoing_rx) = mpsc::channel();
        self.spawner
            .spawn(track_err!(connected)
                       .and_then(move |stream| {
                                     TcpHandleClientLoop::new(client,
                                                              stream,
                                                              incoming_tx0,
                                                              outgoing_rx)
                                 })
                       .then(move |result| {
                                 let _ = incoming_tx1.send(IncomingCommand::Exit(client, result));
                                 Ok(())
                             }));
        self.clients.insert(client, outgoing_tx);
    }
    fn handle_incoming_command(&mut self,
                               command: IncomingCommand)
                               -> Option<(SocketAddr, Result<RawMessage>)> {
        match command {
            IncomingCommand::Recv(addr, message) => Some((addr, message)),
            IncomingCommand::Exit(addr, result) => {
                self.clients.remove(&addr);
                if let Err(e) = result {
                    Some((addr, Err(e)))
                } else {
                    None
                }
            }
        }
    }
}
impl Transport for TcpServerTransport {}
impl MessageSink for TcpServerTransport {}
impl MessageStream for TcpServerTransport {}
impl Sink for TcpServerTransport {
    type SinkItem = MessageSinkItem;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let (peer, message, link) = item;
        let link = if let Some(outgoing_tx) = self.clients.get(&peer) {
            match outgoing_tx.send(OutgoingCommand::Send(message, link)) {
                Ok(_) => None,
                Err(e) => {
                    let SendError(OutgoingCommand::Send(_, link)) = e;
                    link
                }
            }
        } else {
            link
        };
        if let Some(link) = link {
            let e = ErrorKind::Other.cause(format!("No such client found: {}", peer));
            link.exit(Err(track!(e)));
        }
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
}
impl Stream for TcpServerTransport {
    type Item = (SocketAddr, Result<RawMessage>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match track_try!(self.listener.poll()) {
                Async::NotReady => {}
                Async::Ready(None) => return Ok(Async::Ready(None)),
                Async::Ready(Some((connected, client))) => {
                    self.handle_new_connection(client, connected);
                    continue;
                }
            }
            match track_try!(self.incoming_rx.poll().map_err(|()| ErrorKind::Other)) {
                Async::NotReady => return Ok(Async::NotReady),
                Async::Ready(None) => unreachable!(),
                Async::Ready(Some(command)) => {
                    if let Some(item) = self.handle_incoming_command(command) {
                        return Ok(Async::Ready(Some(item)));
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct TcpHandleClientLoop {
    peer: SocketAddr,
    transport: TcpClientTransport,
    incoming_tx: mpsc::Sender<IncomingCommand>,
    outgoing_rx: mpsc::Receiver<OutgoingCommand>,
}
impl TcpHandleClientLoop {
    fn new(peer: SocketAddr,
           stream: TcpStream,
           incoming_tx: mpsc::Sender<IncomingCommand>,
           outgoing_rx: mpsc::Receiver<OutgoingCommand>)
           -> Self {
        TcpHandleClientLoop {
            peer: peer,
            transport: TcpClientTransport::new(peer, stream),
            incoming_tx: incoming_tx,
            outgoing_rx: outgoing_rx,
        }
    }
}
impl Future for TcpHandleClientLoop {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            track_try!(self.transport.poll_complete());
            match track_try!(self.outgoing_rx.poll().map_err(|()| ErrorKind::Other)) {
                Async::NotReady => {}
                Async::Ready(None) => return Ok(Async::Ready(())),
                Async::Ready(Some(command)) => {
                    let OutgoingCommand::Send(message, link) = command;
                    track_try!(self.transport.start_send((self.peer, message, link)));
                    continue;
                }
            }
            match track_try!(self.transport.poll()) {
                Async::NotReady => return Ok(Async::NotReady),
                Async::Ready(None) => return Ok(Async::Ready(())),
                Async::Ready(Some((peer, message))) => {
                    assert_eq!(peer, self.peer);
                    let command = IncomingCommand::Recv(peer, message);
                    let _ = self.incoming_tx.send(command);
                }
            }
        }
    }
}

/// TCP based client-side implementation of [Transport](trait.Transport.html) trait.
///
/// This can only communicate with pre-connected peer (server).
#[derive(Debug)]
pub struct TcpClientTransport {
    sink: TcpMessageSink,
    stream: TcpMessageStream,
}
impl TcpClientTransport {
    /// Makes a new `TcpClientTransport` instance.
    pub fn new(peer: SocketAddr, stream: TcpStream) -> Self {
        TcpClientTransport {
            sink: TcpMessageSink::new(peer, stream.clone()),
            stream: TcpMessageStream::new(peer, stream),
        }
    }
}
impl Transport for TcpClientTransport {}
impl MessageSink for TcpClientTransport {}
impl MessageStream for TcpClientTransport {}
impl Sink for TcpClientTransport {
    type SinkItem = MessageSinkItem;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sink.start_send(item)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.poll_complete()
    }
}
impl Stream for TcpClientTransport {
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
                        self.state = Some(Ok(stream));
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
        track_assert_eq!(addr, self.peer, ErrorKind::Invalid);
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
