use std::net::SocketAddr;
use fibers::Spawn;
use futures::{Future, Poll};

use {Client, Error};
use transport::{UdpTransport, UdpTransportBuilder};
use transport::futures::UdpTransportBind;
use message::RawMessage;
use super::BaseClient;

pub type UdpCall =
    <BaseClient<UdpTransport> as Client>::CallRaw;
pub type UdpCast =
    <BaseClient<UdpTransport> as Client>::CastRaw;

// TODO: UdpClientBuilder
pub struct UdpClient(BaseClient<UdpTransport>);
impl UdpClient {
    pub fn new<S: Spawn>(spawner: S, server: SocketAddr) -> InitUdpClient<S> {
        InitUdpClient {
            bind: UdpTransportBuilder::new().finish(),
            spawner: spawner,
            server: server,
        }
    }
}
impl Client for UdpClient {
    type CallRaw = UdpCall;
    type CastRaw = UdpCast;
    fn call_raw(&mut self, message: RawMessage) -> Self::CallRaw {
        self.0.call_raw(message)
    }
    fn cast_raw(&mut self, message: RawMessage) -> Self::CastRaw {
        self.0.cast_raw(message)
    }
}

pub struct InitUdpClient<S> {
    spawner: S,
    bind: UdpTransportBind,
    server: SocketAddr,
}
impl<S: Spawn> Future for InitUdpClient<S> {
    type Item = UdpClient;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_try!(self.bind.poll())
            .map(|transport| UdpClient(BaseClient::new(&self.spawner, self.server, transport))))
    }
}
