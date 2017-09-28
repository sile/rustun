use std::net::SocketAddr;
use fibers::Spawn;

use Client;
use transport::UdpTransport;
use message::RawMessage;
use super::BaseClient;

/// `Future` that handle a request/response transaction issued by `UdpClient`.
pub type UdpCallRaw = <BaseClient<UdpTransport> as Client>::CallRaw;

/// `Future` that handle a indication transaction issued by `UdpClient`.
pub type UdpCastRaw = <BaseClient<UdpTransport> as Client>::CastRaw;

/// A [Client](trait.Client.html) trait implementation which
/// uses [UdpTransport](../transport/struct.UdpTransport.html) as the transport layer.
pub struct UdpClient(BaseClient<UdpTransport>);
impl UdpClient {
    /// Makes a future that results in a `UdpClient` instance which communicates with `server`.
    ///
    /// If you want to customize the settings of `UdpClient`,
    /// please use `with_transport` function instead.
    pub fn new<S: Spawn>(spawner: &S, server: SocketAddr) -> Self {
        Self::with_transport(spawner, server, UdpTransport::new())
    }

    /// Makes a future that results in a `UdpClient` instance which communicates with `server`.
    ///
    /// The resulting `UdpClient` uses `transport` as the UDP transport layer.
    pub fn with_transport<S: Spawn>(
        spawner: &S,
        server: SocketAddr,
        transport: UdpTransport,
    ) -> Self {
        let inner = BaseClient::new(spawner, server, transport);
        UdpClient(inner)
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
