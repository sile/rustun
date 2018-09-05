//! STUN client related components.
use futures::Stream;
use stun_codec::{Attribute, Method};

use message::{Indication, Request, Response};
use {AsyncResult, Error};

pub use self::tcp::TcpClient;
pub use self::udp::UdpClient;

mod tcp;
mod timeout_queue;
mod udp;

pub trait Client<M, A>: Stream<Item = Indication<M, A>, Error = Error>
where
    M: Method,
    A: Attribute,
{
    fn call(&mut self, request: Request<M, A>) -> AsyncResult<Response<M, A>>;
    fn cast(&mut self, indication: Indication<M, A>);
}
