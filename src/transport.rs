//! Transport layer.
use bytecodec::io::{BufferedIo, IoDecodeExt, IoEncodeExt};
use bytecodec::{Decode, DecodeExt, Encode, EncodeExt, Eos};
use fibers::net::futures::{RecvFrom, SendTo};
use fibers::net::{TcpStream, UdpSocket};
use futures::{Async, Future};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::time::Duration;
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

pub trait UnreliableTransport: Transport {}

pub trait StunTransport<A>:
    Transport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>
where
    A: Attribute,
{
    fn finish_transaction(&mut self, transaction_id: TransactionId);
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

    pub fn stream_ref(&self) -> &TcpStream {
        self.stream.stream_ref()
    }

    pub fn stream_mut(&mut self) -> &mut TcpStream {
        self.stream.stream_mut()
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    pub fn outgoing_queue_len(&self) -> usize {
        self.outgoing_queue.len() + if self.encoder.is_idle() { 0 } else { 1 }
    }

    pub fn decoder_ref(&self) -> &D {
        &self.decoder
    }

    pub fn decoder_mut(&mut self) -> &mut D {
        &mut self.decoder
    }

    pub fn encoder_ref(&self) -> &E {
        &self.encoder
    }

    pub fn encoder_mut(&mut self) -> &mut E {
        &mut self.encoder
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
            stream: BufferedIo::new(stream, 8192, 8192),
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
    fn finish_transaction(&mut self, _transaction_id: TransactionId) {}
}

#[derive(Debug, Clone)]
pub struct UdpTransporterBuilder {
    recv_buf_size: usize,
}
impl UdpTransporterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recv_buf_size(&mut self, size: usize) -> &mut Self {
        self.recv_buf_size = size;
        self
    }

    pub fn bind<D, E>(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Item = UdpTransporter<D, E>, Error = Error>
    where
        D: Decode + Default,
        E: Encode + Default,
    {
        let builder = self.clone();
        UdpSocket::bind(addr)
            .map(move |socket| builder.from_socket(socket))
            .map_err(|e| track!(Error::from(e)))
    }

    pub fn from_socket<D, E>(&self, socket: UdpSocket) -> UdpTransporter<D, E>
    where
        D: Decode + Default,
        E: Encode + Default,
    {
        let recv_from = socket.clone().recv_from(vec![0; self.recv_buf_size]);
        UdpTransporter {
            socket,
            decoder: D::default(),
            encoder: E::default(),
            outgoing_queue: VecDeque::new(),
            send_to: None,
            recv_from,
            last_error: None,
        }
    }
}
impl Default for UdpTransporterBuilder {
    fn default() -> Self {
        UdpTransporterBuilder {
            recv_buf_size: constants::DEFAULT_MAX_MESSAGE_SIZE,
        }
    }
}

#[derive(Debug)]
pub struct UdpTransporter<D: Decode, E: Encode> {
    socket: UdpSocket,
    decoder: D,
    encoder: E,
    outgoing_queue: VecDeque<(SocketAddr, E::Item)>,
    send_to: Option<SendTo<Vec<u8>>>,
    recv_from: RecvFrom<Vec<u8>>,
    last_error: Option<Error>,
}
impl<D, E> UdpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    pub fn bind(addr: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        UdpTransporterBuilder::default().bind(addr)
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
        UdpTransporterBuilder::default().from_socket(f)
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

#[derive(Debug, Clone)]
pub struct RetransmitTransporterBuilder {
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}
impl RetransmitTransporterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finish<A, T>(&self, inner: T) -> RetransmitTransporter<A, T>
    where
        A: Attribute,
        T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
    {
        RetransmitTransporter {
            inner,
            _phantom: PhantomData,
            current_rto: self.rto,
            cached_rto: self.rto,
            rto: self.rto,
            rto_cache_duration: self.rto_cache_duration,
            min_transaction_interval: self.min_transaction_interval,
            max_outstanding_transactions: self.max_outstanding_transactions,
        }
    }
}
impl Default for RetransmitTransporterBuilder {
    fn default() -> Self {
        RetransmitTransporterBuilder {
            rto: Duration::from_millis(constants::DEFAULT_RTO_MS),
            rto_cache_duration: Duration::from_millis(constants::DEFAULT_RTO_CACHE_DURATION_MS),
            min_transaction_interval: Duration::from_millis(
                constants::DEFAULT_MIN_TRANSACTION_INTERVAL_MS,
            ),
            max_outstanding_transactions: constants::DEFAULT_MAX_OUTSTANDING_TRANSACTIONS,
        }
    }
}

#[derive(Debug)]
pub struct RetransmitTransporter<A, T> {
    inner: T,
    _phantom: PhantomData<A>,
    current_rto: Duration,
    cached_rto: Duration,
    rto: Duration,
    rto_cache_duration: Duration,
    min_transaction_interval: Duration,
    max_outstanding_transactions: usize,
}
impl<A, T> RetransmitTransporter<A, T>
where
    A: Attribute,
    T: UnreliableTransport<Decoder = MessageDecoder<A>, Encoder = MessageEncoder<A>>,
{
    pub fn new(inner: T) -> Self {
        RetransmitTransporterBuilder::new().finish(inner)
    }

    pub fn inner_ref(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
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
    fn finish_transaction(&mut self, _transaction_id: TransactionId) {
        panic!("TODO")
    }
}
