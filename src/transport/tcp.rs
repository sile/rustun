use bytecodec::io::{BufferedIo, IoDecodeExt, IoEncodeExt};
use bytecodec::{Decode, Encode, Eos};
use fibers::net::TcpStream;
use futures::Future;
use std::collections::VecDeque;
use std::net::SocketAddr;
use stun_codec::{Attribute, MessageDecoder, MessageEncoder, TransactionId};
use trackable::error::ErrorKindExt;

use super::{StunTransport, Transport};
use {Error, ErrorKind, Result};

/// An implementation of [`Transport`] that uses TCP as the transport layer.
///
/// [`Transport`]: ./trait.Transport.html
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
    /// Starts connecting to the given peer and
    /// will return a new `TcpTransporter` instance if the connect operation is succeeded.
    pub fn connect(peer: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        TcpStream::connect(peer)
            .map(move |stream| Self::from((peer, stream)))
            .map_err(|e| track!(Error::from(e)))
    }

    /// Makes a new `TcpTransporter` instance from the given TCP stream.
    ///
    /// # Errors
    ///
    /// If `stream.peer_addr()` returns an error, this function will return an `ErrorKind::Other` error.
    pub fn from_stream(stream: TcpStream) -> Result<Self> {
        let peer = track!(stream.peer_addr().map_err(Error::from))?;
        Ok(Self::from((peer, stream)))
    }

    /// Returns the address of the connected peer.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    /// Returns the number of unsent messages in the queue of the instance.
    pub fn message_queue_len(&self) -> usize {
        self.outgoing_queue.len() + if self.encoder.is_idle() { 0 } else { 1 }
    }

    /// Returns a reference to the TCP stream being used by the instance.
    pub fn stream_ref(&self) -> &TcpStream {
        self.stream.stream_ref()
    }

    /// Returns a mutable reference to the TCP stream being used by the instance.
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        self.stream.stream_mut()
    }

    /// Returns a reference to the decoder being used by the instance.
    pub fn decoder_ref(&self) -> &D {
        &self.decoder
    }

    /// Returns a mutable reference to the decoder being used by the instance.
    pub fn decoder_mut(&mut self) -> &mut D {
        &mut self.decoder
    }

    /// Returns a reference to the encoder being used by the instance.
    pub fn encoder_ref(&self) -> &E {
        &self.encoder
    }

    /// Returns a mutable reference to the encoder being used by the instance.
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
    fn finish_transaction(&mut self, _peer: SocketAddr, _transaction_id: TransactionId) {}
}
