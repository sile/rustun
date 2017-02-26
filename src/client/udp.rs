use std::net::SocketAddr;
use fibers::Spawn;
use futures::{Future, Poll};

use {Client, Error};
use transport::{UdpTransport, UdpTransportBuilder};
use transport::futures::UdpTransportBind;
use message::RawMessage;
use super::BaseClient;

/// `Future` that handle a request/response transaction issued by `UdpClient`.
pub type UdpCallRaw =
    <BaseClient<UdpTransport> as Client>::CallRaw;

/// `Future` that handle a indication transaction issued by `UdpClient`.
pub type UdpCastRaw =
    <BaseClient<UdpTransport> as Client>::CastRaw;

/// A [Client](trait.Client.html) trait implementation which
/// uses [UdpTransport](../transport/struct.UdpTransport.html) as the transport layer.
pub struct UdpClient(BaseClient<UdpTransport>);
impl UdpClient {
    /// Makes a future that results in a `UdpClient` instance which communicates with `server`.
    ///
    /// If you want to customize the settings of `UdpClient`,
    /// please use `from_builder` function instead.
    pub fn new<S: Spawn>(spawner: S, server: SocketAddr) -> InitUdpClient<S> {
        Self::from_builder(spawner, server, &UdpTransportBuilder::new())
    }

    /// Makes a future that results in a `UdpClient` instance which communicates with `server`.
    ///
    /// The resulting `UdpClient` uses a `UdpTransport` instance
    /// which have the settings specified by `builder`.
    pub fn from_builder<S: Spawn>(spawner: S,
                                  server: SocketAddr,
                                  builder: &UdpTransportBuilder)
                                  -> InitUdpClient<S> {
        InitUdpClient {
            bind: builder.finish(),
            spawner: spawner,
            server: server,
        }
    }
}
impl Client for UdpClient {
    type CallRaw = UdpCallRaw;
    type CastRaw = UdpCastRaw;
    fn call_raw(&mut self, message: RawMessage) -> Self::CallRaw {
        self.0.call_raw(message)
    }
    fn cast_raw(&mut self, message: RawMessage) -> Self::CastRaw {
        self.0.cast_raw(message)
    }
}

/// `Future` that results in a `UdpClient` instance.
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
