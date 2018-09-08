use bytecodec::{Decode, DecodeExt, Encode, EncodeExt};
use fibers::net::futures::{RecvFrom, SendTo};
use fibers::net::UdpSocket;
use futures::{Async, Future};
use std::collections::VecDeque;
use std::net::SocketAddr;

use super::{Transport, UnreliableTransport};
use constants;
use {Error, Result};

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
            .map(move |socket| builder.finish(socket))
            .map_err(|e| track!(Error::from(e)))
    }

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

    pub fn message_queue_len(&self) -> usize {
        self.outgoing_queue.len() + if self.encoder.is_idle() { 0 } else { 1 }
    }

    pub fn socket_ref(&self) -> &UdpSocket {
        &self.socket
    }

    pub fn socket_mut(&mut self) -> &mut UdpSocket {
        &mut self.socket
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
