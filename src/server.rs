use std::io;
use std::net::SocketAddr;
use fibers::net::UdpSocket;
use futures::{Future, BoxFuture};

#[derive(Debug)]
pub struct StunServer {
    pub socket: UdpSocket,
}
impl StunServer {
    pub fn start(bind_addr: SocketAddr) -> Start {
        UdpSocket::bind(bind_addr).map(|socket| StunServer { socket: socket }).boxed()
    }
}

pub type Start = BoxFuture<StunServer, io::Error>;
