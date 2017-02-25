use std::net::SocketAddr;
use fibers::Spawn;
use futures::{Future, Poll};

use {Method, Attribute, Client, Error};
use transport::{UdpTransport, UdpTransportBuilder};
use transport::futures::UdpTransportBind;
use message::{Indication, Request};
use super::BaseClient;

pub type UdpCall<M, A> =
    <BaseClient<UdpTransport> as Client<M, A>>::Call;
pub type UdpCast<M, A> =
    <BaseClient<UdpTransport> as Client<M, A>>::Cast;

// TODO: UdpClientBuilder
pub struct UdpClient(BaseClient<UdpTransport>);
impl UdpClient {
    pub fn new<S: Spawn>(spawner: S, server: SocketAddr) -> InitUdpClient<S> {
        InitUdpClient {
            bind: UdpTransportBuilder::new().finish(),
            spawner: spawner,
            server: server,
        }
    }
}
impl<M, A> Client<M, A> for UdpClient
    where M: Method,
          A: Attribute
{
    type Call = UdpCall<M, A>;
    type Cast = UdpCast<M, A>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call {
        self.0.call(message)
    }
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast {
        self.0.cast(message)
    }
}

pub struct InitUdpClient<S> {
    spawner: S,
    bind: UdpTransportBind,
    server: SocketAddr,
}
impl<S: Spawn> Future for InitUdpClient<S> {
    type Item = UdpClient;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_try!(self.bind.poll())
            .map(|transport| UdpClient(BaseClient::new(&self.spawner, self.server, transport))))
    }
}
