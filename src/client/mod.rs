//! STUN client related components.
use bytecodec::marker::Never;
use fibers::Spawn;
use futures::Future;
use stun_codec::{Attribute, Method};

use message::{Indication, Request, Response};
use {AsyncReply, AsyncResult, Error};

pub use self::handle::ClientHandle;
pub use self::tcp::TcpClient;
pub use self::udp::UdpClient;

mod handle;
mod tcp;
mod timeout_queue;
mod udp;

pub trait Client<M, A>: Future<Item = Never, Error = Error>
where
    M: Method,
    A: Attribute,
{
    fn call(&mut self, request: Request<M, A>) -> AsyncResult<Response<M, A>> {
        let (reply, result) = AsyncResult::new();
        self.call_with_reply(request, reply);
        result
    }
    fn call_with_reply(&mut self, request: Request<M, A>, reply: AsyncReply<Response<M, A>>);
    fn cast(&mut self, indication: Indication<M, A>);
    fn into_handle<S: Spawn>(self, spawner: S) -> ClientHandle<M, A>
    where
        M: Send + 'static,
        A: Send + 'static,
        Self: Sized + Send + 'static,
    {
        let (handle, future) = ClientHandle::new(self);
        spawner.spawn(future);
        handle
    }
}
