use fibers::net::UdpSocket;
use fibers::Spawn;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

use server::futures::BaseServerLoop;
use server::BaseServer;
use transport::{UdpTransport, UdpTransportBuilder};
use HandleMessage;

/// UDP STUN server.
#[derive(Debug)]
pub struct UdpServer {
    builder: UdpTransportBuilder,
}
impl UdpServer {
    /// Makes a new `UdpServer` instance which will bind to `bind_addr`.
    pub fn new(bind_addr: SocketAddr) -> Self {
        let mut builder = UdpTransportBuilder::new();
        builder.bind_addr(bind_addr);
        UdpServer { builder: builder }
    }

    /// Makes a new `UdpServer` instance which uses the `socket` to communiate with clients.
    pub fn with_socket(socket: UdpSocket) -> Self {
        let builder = UdpTransportBuilder::with_socket(socket);
        UdpServer { builder: builder }
    }

    /// Starts the UDP server with `handler`.
    pub fn start<S, H>(&self, spawner: S, handler: H) -> UdpServerLoop<H>
    where
        S: Spawn + Send + 'static,
        H: HandleMessage,
    {
        let transport = self.builder.finish();
        BaseServer::start(spawner, transport, handler)
    }
}
impl Deref for UdpServer {
    type Target = UdpTransportBuilder;
    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}
impl DerefMut for UdpServer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.builder
    }
}

/// `Future` that represent the loop of a UDP server for handling transactions issued by clients.
pub type UdpServerLoop<H> = BaseServerLoop<UdpTransport, H>;
