use std::net::SocketAddr;
use slog::Logger;
// use fibers::net::UdpSocket;
use futures::{self, Future, BoxFuture};

use Error;

#[derive(Debug)]
pub struct UdpServer {
    bind_addr: SocketAddr,
    logger: Option<Logger>,
}
impl UdpServer {
    pub fn new(bind_addr: SocketAddr) -> Self {
        UdpServer {
            bind_addr: bind_addr,
            logger: None,
        }
    }
    pub fn set_logger(&mut self, logger: Logger) -> &mut Self {
        self.logger = Some(logger);
        self
    }
    pub fn start(&mut self) -> BoxFuture<(), Error> {
        futures::finished(()).boxed()
    }
}
