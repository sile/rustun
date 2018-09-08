use bytecodec::marker::Never;
use futures::{Future, Poll};
use std::net::SocketAddr;

use Error;

#[derive(Debug)]
pub struct UdpServer<H> {
    message_handler: H,
}
impl<H> UdpServer<H> {
    pub fn start(_bind_addr: SocketAddr, _handler: H) -> Self {
        panic!()
    }
}
impl<H> Future for UdpServer<H> {
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        panic!()
    }
}

#[derive(Debug)]
pub struct TcpServer<H> {
    message_handler_factory: H,
}
impl<H> TcpServer<H> {
    pub fn start(_bind_addr: SocketAddr, _handler_factory: H) -> Self {
        panic!()
    }
}
impl<H> Future for TcpServer<H> {
    type Item = Never;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        panic!()
    }
}
