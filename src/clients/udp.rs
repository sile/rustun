use std::net::SocketAddr;
use fibers::Spawn;
use fibers::net::UdpSocket;

use {Method, Attribute, Client};
use transport::{UdpSender, UdpReceiver};
use message::{Indication, Request};
use super::BaseClient;

pub type UdpCall<M, A> =
    <BaseClient<UdpSender, UdpReceiver> as Client<M, A>>::Call;

pub type UdpCast<M, A> =
    <BaseClient<UdpSender, UdpReceiver> as Client<M, A>>::Cast;

#[derive(Debug)]
pub struct UdpClient {
    base: BaseClient<UdpSender, UdpReceiver>,
}
impl UdpClient {
    pub fn new<T: Spawn>(spawner: T, socket: UdpSocket, server: SocketAddr) -> Self {
        let sender = UdpSender::new(socket.clone(), server);
        let receiver = UdpReceiver::new(socket);
        UdpClient { base: BaseClient::new(spawner, sender, receiver) }
    }
}
impl<M, A> Client<M, A> for UdpClient
    where M: Method,
          A: Attribute
{
    type Call = UdpCall<M, A>;
    type Cast = UdpCast<M, A>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call {
        self.base.call(message)
    }
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast {
        self.base.cast(message)
    }
}
