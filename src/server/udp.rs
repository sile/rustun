use std::net::SocketAddr;
use fibers::Spawn;

use HandleMessage;
use transport::{UdpTransport, UdpTransportBuilder};
use server::BaseServer;
use server::futures::BaseServerLoop;

/// UDP STUN server.
#[derive(Debug)]
pub struct UdpServer {
    bind_addr: SocketAddr,
}
impl UdpServer {
    /// Makes a new `UdpServer` instance which will bind to `bind_addr`.
    pub fn new(bind_addr: SocketAddr) -> Self {
        UdpServer { bind_addr: bind_addr }
    }

    /// Starts the UDP server with `handler`.
    pub fn start<S, H>(&self, spawner: S, handler: H) -> UdpServerLoop<H>
        where S: Spawn + Send + 'static,
              H: HandleMessage
    {
        let transport = UdpTransportBuilder::new().bind_addr(self.bind_addr).finish();
        BaseServer::start(spawner, transport, handler)
    }
}

/// `Future` that represent the loop of a UDP server for handling transactions issued by clients.
pub type UdpServerLoop<H> = BaseServerLoop<UdpTransport, H>;
