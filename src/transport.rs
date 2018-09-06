//! Transport layer.
use bytecodec::io::{BufferedIo, IoDecodeExt, IoEncodeExt};
use bytecodec::{self, Decode, DecodeExt, Encode, EncodeExt, Eos};
use fibers::net::futures::{RecvFrom, SendTo};
use fibers::net::{TcpStream, UdpSocket};
use futures::{Async, Future};
use std::collections::VecDeque;
use std::net::SocketAddr;
use stun_codec::{Attribute, MessageDecoder, MessageEncoder, TransactionId};
use trackable::error::ErrorKindExt;

use constants;
use {Error, ErrorKind, Result};

pub trait Transport {
    type Decoder: Decode;
    type Encoder: Encode;

    fn send(&mut self, peer: SocketAddr, item: <Self::Encoder as Encode>::Item);
    fn recv(&mut self) -> Option<(SocketAddr, <Self::Decoder as Decode>::Item)>;
    fn run_once(&mut self) -> Result<bool>;
}

#[derive(Debug)]
pub enum RecvResult<T> {
    None,
    Some {
        peer: SocketAddr,
        item: T,
    },
    DecodeError {
        peer: SocketAddr,
        error: bytecodec::Error,
    },
}

pub trait UnreliableTransport: Transport {}

pub trait StunTransport<A>:
    Transport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>
where
    A: Attribute,
{
    fn cancel_retransmission(&mut self, transaction_id: TransactionId);
}

#[derive(Debug)]
pub struct TcpTransporter<D: Decode, E: Encode> {
    stream: BufferedIo<TcpStream>,
    peer: SocketAddr,
    decoder: D,
    encoder: E,
    outgoing_queue: VecDeque<E::Item>,
    last_error: Option<Error>,
}
impl<D, E> TcpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    pub fn connect(peer: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        TcpStream::connect(peer)
            .map(move |stream| Self::from((peer, stream)))
            .map_err(|e| track!(Error::from(e)))
    }

    fn start_send(&mut self, item: E::Item) -> Result<()> {
        if self.encoder.is_idle() {
            track!(self.encoder.start_encoding(item))?;
        } else {
            self.outgoing_queue.push_back(item);
        }
        track!(self.poll_send())?;
        Ok(())
    }

    fn poll_send(&mut self) -> Result<()> {
        while !self.stream.is_eos() {
            track!(self.stream.execute_io())?;
            track!(
                self.encoder
                    .encode_to_write_buf(self.stream.write_buf_mut())
            )?;
            if self.encoder.is_idle() {
                if let Some(item) = self.outgoing_queue.pop_front() {
                    track!(self.encoder.start_encoding(item))?;
                } else {
                    break;
                }
            }
            if self.stream.would_block() {
                break;
            }
        }
        Ok(())
    }

    fn poll_recv(&mut self) -> Result<Option<(SocketAddr, D::Item)>> {
        while self.stream.is_eos() {
            track!(self.stream.execute_io())?;
            track!(
                self.decoder
                    .decode_from_read_buf(self.stream.read_buf_mut())
            )?;
            if self.decoder.is_idle() {
                let item = track!(self.decoder.finish_decoding())?;
                return Ok(Some((self.peer, item)));
            }
            if self.stream.would_block() {
                break;
            }
        }
        Ok(None)
    }
}
impl<D, E> From<(SocketAddr, TcpStream)> for TcpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    fn from((peer, stream): (SocketAddr, TcpStream)) -> Self {
        let _ = stream.set_nodelay(true);
        TcpTransporter {
            stream: BufferedIo::new(stream, 4096, 4096),
            peer,
            decoder: D::default(),
            encoder: E::default(),
            outgoing_queue: VecDeque::new(),
            last_error: None,
        }
    }
}
impl<D, E> Transport for TcpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    type Decoder = D;
    type Encoder = E;

    fn send(&mut self, peer: SocketAddr, item: E::Item) {
        if self.last_error.is_some() {
            return;
        }
        if peer != self.peer {
            let e = ErrorKind::InvalidInput.cause(format!(
                "Unexpected destination peer: actual={}, expected={}",
                peer, self.peer
            ));
            self.last_error = Some(e.into());
            return;
        }
        self.last_error = self.start_send(item).err();
    }

    fn recv(&mut self) -> Option<(SocketAddr, D::Item)> {
        if self.last_error.is_some() {
            return None;
        }
        match self.poll_recv() {
            Err(e) => {
                self.last_error = Some(e);
                None
            }
            Ok(item) => item,
        }
    }

    fn run_once(&mut self) -> Result<bool> {
        if let Some(e) = self.last_error.take() {
            return Err(track!(e));
        }
        track!(self.poll_send())?;
        if self.stream.is_eos() {
            track!(self.decoder.decode(&[], Eos::new(true)))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
impl<A> StunTransport<A> for TcpTransporter<MessageDecoder<A>, MessageEncoder<A>>
where
    A: Attribute,
{
    fn cancel_retransmission(&mut self, _transaction_id: TransactionId) {}
}

#[derive(Debug)]
pub struct UdpTransporter<D: Decode, E: Encode> {
    socket: UdpSocket,
    decoder: D,
    encoder: E,
    outgoing_queue: VecDeque<(SocketAddr, E::Item)>,
    send_to: Option<SendTo<Vec<u8>>>,
    recv_from: RecvFrom<Vec<u8>>, // TODO: parameter
    last_error: Option<Error>,
}
impl<D, E> UdpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    pub fn bind(addr: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        UdpSocket::bind(addr)
            .map(Self::from)
            .map_err(|e| track!(Error::from(e)))
    }

    fn poll_send(&mut self) -> Result<()> {
        while track!(
            self.send_to
                .poll()
                .map_err(|(_, _, e)| track!(Error::from(e)))
        )?.is_ready()
        {
            if let Some((peer, item)) = self.outgoing_queue.pop_front() {
                let bytes = track!(self.encoder.encode_into_bytes(item))?;
                self.send_to = Some(self.socket.clone().send_to(bytes, peer));
            } else {
                self.send_to = None;
                break;
            }
        }
        Ok(())
    }

    fn poll_recv(&mut self) -> Result<Option<(SocketAddr, D::Item)>> {
        while let Async::Ready((socket, buf, size, peer)) = self
            .recv_from
            .poll()
            .map_err(|(_, _, e)| track!(Error::from(e)))?
        {
            let item = track!(self.decoder.decode_from_bytes(&buf[..size]))?;
            self.recv_from = socket.recv_from(buf);
            return Ok(Some((peer, item)));
        }
        Ok(None)
    }
}
impl<D, E> From<UdpSocket> for UdpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    fn from(f: UdpSocket) -> Self {
        let recv_from = f
            .clone()
            .recv_from(vec![0; constants::DEFAULT_MAX_MESSAGE_SIZE]);
        UdpTransporter {
            socket: f,
            decoder: D::default(),
            encoder: E::default(),
            outgoing_queue: VecDeque::new(),
            send_to: None,
            recv_from,
            last_error: None,
        }
    }
}
impl<D, E> Transport for UdpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    type Decoder = D;
    type Encoder = E;

    fn send(&mut self, peer: SocketAddr, item: E::Item) {
        if self.last_error.is_some() {
            return;
        }
        self.outgoing_queue.push_back((peer, item));
        self.last_error = self.poll_send().err();
    }

    fn recv(&mut self) -> Option<(SocketAddr, D::Item)> {
        if self.last_error.is_some() {
            return None;
        }
        match self.poll_recv() {
            Err(e) => {
                self.last_error = Some(e);
                None
            }
            Ok(item) => item,
        }
    }

    fn run_once(&mut self) -> Result<bool> {
        if let Some(e) = self.last_error.take() {
            return Err(track!(e));
        }
        track!(self.poll_send())?;
        Ok(false)
    }
}
impl<D, E> UnreliableTransport for UdpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{}

#[derive(Debug)]
pub struct RetransmitTransporter<A, T> {
    inner: T,
    _phatom: ::std::marker::PhantomData<A>,
}
impl<A, T> RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
{
    pub fn new(inner: T) -> Self {
        RetransmitTransporter {
            inner,
            _phatom: Default::default(),
        }
    }
}
impl<A, T> Transport for RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
{
    type Decoder = MessageDecoder<A>;
    type Encoder = MessageEncoder<A>;

    // TODO:
    fn send(&mut self, peer: SocketAddr, item: <Self::Encoder as Encode>::Item) {
        self.inner.send(peer, item);
    }
    fn recv(&mut self) -> Option<(SocketAddr, <Self::Decoder as Decode>::Item)> {
        self.inner.recv()
    }
    fn run_once(&mut self) -> Result<bool> {
        track!(self.inner.run_once())
    }
}
impl<A, T> StunTransport<A> for RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
{
    fn cancel_retransmission(&mut self, _transaction_id: TransactionId) {
        panic!("TODO")
    }
}
