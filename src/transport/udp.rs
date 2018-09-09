use bytecodec::{Decode, DecodeExt, Encode, EncodeExt};
use fibers::net::futures::{RecvFrom, SendTo};
use fibers::net::UdpSocket;
use futures::{Async, Future};
use std::collections::VecDeque;
use std::net::SocketAddr;

use super::{Transport, UnreliableTransport};
use {Error, Result};

/// [`UdpTransporter`] builder.
///
/// [`UdpTransporter`]: ./struct.UdpTransporter.html
#[derive(Debug, Clone)]
pub struct UdpTransporterBuilder {
    recv_buf_size: usize,
}
impl UdpTransporterBuilder {
    /// The default maximum size of a message.
    ///
    /// > All STUN messages sent over UDP SHOULD be less than the path MTU, if
    /// > known.  If the path MTU is unknown, messages SHOULD be the smaller of
    /// > 576 bytes and the first-hop MTU for IPv4 [RFC1122] and 1280 bytes for
    /// > IPv6 [RFC2460].  This value corresponds to the overall size of the IP
    /// > packet.  Consequently, for IPv4, the actual STUN message would need
    /// > to be less than **548 bytes** (576 minus 20-byte IP header, minus 8-byte
    /// > UDP header, assuming no IP options are used).
    /// >
    /// > [RFC 5389 -- 7.1. Forming a Request or an Indication]
    ///
    /// [RFC 5389 -- 7.1. Forming a Request or an Indication]: https://tools.ietf.org/html/rfc5389#section-7.1
    pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 548;

    /// Makes a new `UdpTransporterBuilder` instance with the default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the byte size of the receive buffer of the resulting instance.
    ///
    /// The default value is `DEFAULT_MAX_MESSAGE_SIZE`.
    pub fn recv_buf_size(&mut self, size: usize) -> &mut Self {
        self.recv_buf_size = size;
        self
    }

    /// Starts binding to the specified address and will makes
    /// a new `UdpTransporter` instance if the operation is succeeded.
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
            .map(move |socket| builder.finish(socket))
            .map_err(|e| track!(Error::from(e)))
    }

    /// Makes a new `UdpTransporter` instance with the given settings.
    pub fn finish<D, E>(&self, socket: UdpSocket) -> UdpTransporter<D, E>
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
            recv_buf_size: Self::DEFAULT_MAX_MESSAGE_SIZE,
        }
    }
}

/// An implementation of [`Transport`] that uses UDP as the transport layer.
///
/// [`Transport`]: ./trait.Transport.html
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
    /// Starts binding to the specified address and will makes
    /// a new `UdpTransporter` instance if the operation is succeeded.
    ///
    /// This is equivalent to `UdpTransporterBuilder::default().bind(addr)`.
    pub fn bind(addr: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        UdpTransporterBuilder::default().bind(addr)
    }

    /// Returns the number of unsent messages in the queue of the instance.
    pub fn message_queue_len(&self) -> usize {
        self.outgoing_queue.len() + if self.encoder.is_idle() { 0 } else { 1 }
    }

    /// Returns a reference to the UDP socket being used by the instance.
    pub fn socket_ref(&self) -> &UdpSocket {
        &self.socket
    }

    /// Returns a mutable reference to the UDP socket being used by the instance.
    pub fn socket_mut(&mut self) -> &mut UdpSocket {
        &mut self.socket
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
        if let Async::Ready((socket, buf, size, peer)) = self
            .recv_from
            .poll()
            .map_err(|(_, _, e)| track!(Error::from(e)))?
        {
            let item = track!(self.decoder.decode_from_bytes(&buf[..size]))?;
            self.recv_from = socket.recv_from(buf);
            Ok(Some((peer, item)))
        } else {
            Ok(None)
        }
    }
}
impl<D, E> From<UdpSocket> for UdpTransporter<D, E>
where
    D: Decode + Default,
    E: Encode + Default,
{
    fn from(f: UdpSocket) -> Self {
        UdpTransporterBuilder::default().finish(f)
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
