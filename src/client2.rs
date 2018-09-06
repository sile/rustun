use fibers::sync::mpsc;
use fibers::Spawn;
use futures::{Future, Stream};
use std::net::SocketAddr;
use stun_codec::Attribute;

use agent::{Agent, NoopMessageHandler};
use message::{Indication, Request, Response};
use transport::{AnyMethod, StunTransport};
use AsyncResult;

#[derive(Debug, Clone)]
pub struct Client<A> {
    tx: mpsc::Sender<Command<A>>,
}
impl<A: Attribute> Client<A> {
    pub fn new<T, S>(spawner: S, transporter: T) -> Self
    where
        A: Send + 'static,
        T: StunTransport<A> + Send + 'static,
        S: Spawn,
    {
        let (tx, _rx) = mpsc::channel();
        let agent = Agent::new(transporter, NoopMessageHandler::new());
        spawner.spawn(agent.for_each(|_| Ok(())).map_err(|_| panic!("TODO")));
        Client { tx }
    }

    pub fn call<M>(
        &self,
        _peer: SocketAddr,
        _request: Request<M, A>,
    ) -> AsyncResult<Response<M, A>> {
        panic!()
    }

    pub fn cast<M>(&self, _peer: SocketAddr, _indication: Indication<M, A>) {
        panic!()
    }
}

#[derive(Debug)]
pub enum Command<A> {
    Call(Request<AnyMethod, A>),
    Cast(Indication<AnyMethod, A>),
}
