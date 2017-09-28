use std::net::SocketAddr;
use fibers::Spawn;

use HandleMessage;
use transport::TcpServerTransport;
use server::BaseServer;
use server::futures::BaseServerLoop;

/// TCP STUN server.
#[derive(Debug)]
pub struct TcpServer {
    bind_addr: SocketAddr,
}
impl TcpServer {
    /// Makes a new `TcpServer` instance which will bind to `bind_addr`.
    pub fn new(bind_addr: SocketAddr) -> Self {
        TcpServer { bind_addr: bind_addr }
    }

    /// Starts the TCP server with `handler`.
    pub fn start<S, H>(&self, spawner: S, handler: H) -> TcpServerLoop<H>
    where
        S: Spawn + Clone + Send + 'static,
        H: HandleMessage,
    {
        let transport = TcpServerTransport::new(spawner.clone(), self.bind_addr);
        BaseServer::start(spawner, transport, handler)
    }
}

/// `Future` that represent the loop of a TCP server for handling transactions issued by clients.
pub type TcpServerLoop<H> = BaseServerLoop<TcpServerTransport, H>;
