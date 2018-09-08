use fibers::sync::mpsc;
use fibers::Spawn;
use futures::Future;
use std::net::SocketAddr;
use stun_codec::Attribute;

use agent::Agent;
use handler::NoopMessageHandler;
use message::{Indication, Request, Response};
use transport::StunTransport;
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
        spawner.spawn(agent.map_err(|_| panic!("TODO")));
        Client { tx }
    }

    pub fn call(&self, _peer: SocketAddr, _request: Request<A>) -> AsyncResult<Response<A>> {
        panic!()
    }

    pub fn cast(&self, _peer: SocketAddr, _indication: Indication<A>) {
        panic!()
    }
}

#[derive(Debug)]
pub enum Command<A> {
    Call(Request<A>),
    Cast(Indication<A>),
}
