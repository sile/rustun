//! STUN client related components.
use bytecodec::marker::Never;
use futures::Future;
use stun_codec::{Attribute, Method};

use message::{Indication, Request, Response};
use {AsyncResult, Error};

pub use self::tcp::TcpClient;
pub use self::udp::UdpClient;

mod tcp;
mod timeout_queue;
mod udp;

pub trait Client<M, A>: Future<Item = Never, Error = Error>
where
    M: Method,
    A: Attribute,
{
    fn call(&mut self, request: Request<M, A>) -> AsyncResult<Response<M, A>>;
    fn cast(&mut self, indication: Indication<M, A>);
}
