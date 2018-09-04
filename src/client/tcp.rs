use fibers::net::futures::Connect;
use fibers::net::TcpStream;
use fibers::Spawn;
use futures::{Future, Poll};
use std::net::SocketAddr;

use super::BaseClient;
use transport::TcpClientTransport;
use {Client, Error};

/// `Future` that handle a request/response transaction issued by `TcpClient`.
pub type TcpCallRaw = <BaseClient<TcpClientTransport> as Client>::CallRaw;

/// `Future` that handle a indication transaction issued by `TcpClient`.
pub type TcpCastRaw = <BaseClient<TcpClientTransport> as Client>::CastRaw;

/// A [Client](trait.Client.html) trait implementation which
/// uses [TcpClientTransport](../transport/struct.TcpClientTransport.html) as the transport layer.
pub struct TcpClient(BaseClient<TcpClientTransport>);
impl TcpClient {
    /// Makes a future that results in a `TcpClient` instance which communicates with `server`.
    pub fn new<S: Spawn>(spawner: S, server: SocketAddr) -> InitTcpClient<S> {
        InitTcpClient {
            spawner: spawner,
            server: server,
            connect: TcpStream::connect(server),
        }
    }
}
impl Client for TcpClient {
    type CallRaw = TcpCallRaw;
    type CastRaw = TcpCastRaw;
    fn call_raw(&mut self, message: RawMessage) -> Self::CallRaw {
        self.0.call_raw(message)
    }
    fn cast_raw(&mut self, message: RawMessage) -> Self::CastRaw {
        self.0.cast_raw(message)
    }
}

/// `Future` that results in a `TcpClient` instance.
pub struct InitTcpClient<S> {
    spawner: S,
    server: SocketAddr,
    connect: Connect,
}
impl<S: Spawn> Future for InitTcpClient<S> {
    type Item = TcpClient;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_try!(self.connect.poll()).map(|stream| {
            TcpClient(BaseClient::new(
                &self.spawner,
                self.server,
                TcpClientTransport::new(self.server, stream),
            ))
        }))
    }
}
