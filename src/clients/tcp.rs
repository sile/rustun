use fibers::Spawn;
use fibers::net::TcpStream;

use {Method, Attribute, Client};
use transport::{TcpSender, TcpReceiver};
use message::{Indication, Request};
use super::BaseClient;

pub type TcpCall<M, A> =
    <BaseClient<TcpSender, TcpReceiver> as Client<M, A>>::Call;

pub type TcpCast<M, A> =
    <BaseClient<TcpSender, TcpReceiver> as Client<M, A>>::Cast;

#[derive(Debug)]
pub struct TcpClient {
    base: BaseClient<TcpSender, TcpReceiver>,
}
impl TcpClient {
    pub fn new<T: Spawn>(spawner: T, stream: TcpStream) -> Self {
        let sender = TcpSender::new(stream.clone());
        let receiver = TcpReceiver::new(stream);
        TcpClient { base: BaseClient::new(spawner, sender, receiver) }
    }
}
impl<M, A> Client<M, A> for TcpClient
    where M: Method,
          A: Attribute
{
    type Call = TcpCall<M, A>;
    type Cast = TcpCast<M, A>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call {
        self.base.call(message)
    }
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast {
        self.base.cast(message)
    }
}
