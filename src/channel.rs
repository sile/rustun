// use std::net::SocketAddr;
// use stun_codec::{Attribute, Method};

// use message::{Indication, Request, Response};
// use transport::StunTransport;
// use {AsyncResult, Result};

// pub type AsyncResponse<M, A> = AsyncResult<Response<M, A>>;

// pub trait Channel<M, A, T>
// where
//     M: Method,
//     A: Attribute,
//     T: StunTransport<M, A>,
// {
//     fn call(&mut self, peer: SocketAddr, request: Request<M, A>) -> AsyncResponse<M, A>;
//     fn cast(&mut self, peer: SocketAddr, indication: Indication<M, A>);
//     fn reply(&mut self, peer: SocketAddr, response: Response<M, A>);
//     fn recv(&mut self) -> Option<RecvMessage<M, A>>;
//     fn run_once(&mut self) -> Result<bool>;
// }

// #[derive(Debug)]
// pub enum RecvMessage<M, A> {
//     Request {
//         peer: SocketAddr,
//         request: Request<M, A>,
//     },
//     Indication {
//         peer: SocketAddr,
//         indication: Indication<M, A>,
//     },
//     // DecodeError, UnknownMethod, UnexpectedClass, UnknownTransaction
// }

// pub struct Agent<T, H> {
//     transporter: T,
//     handler: T,
// }
// impl<T> Agent<T> {
//     pub fn call<M, A>(&mut self, peer: SocketAddr, request: Request<M, A>) -> AsyncResponse<M, A> {
//         panic!()
//     }

//     pub fn cast<M, A>(&mut self, peer: SocketAddr, indication: Indication<M, A>) {
//         panic!()
//     }

//     pub fn handler(&self) -> &H {
//         panic!()
//     }
// }
// // impl Future
// // impl Drop { let _ = transport.poll(); }

// // TODO: agent
// pub trait HandleMessage {
//     type Method;
//     type Attribute;
//     fn handle_call(&mut self, ctx: Context, request: Request<Self::Method, Self::Attribute>);
//     fn handle_cast(&mut self, ctx: Context, indication: Indication<Self::Method, Self::Attribute>);
//     // handle_error
// }

// pub struct Context {
//     pub fn remote_channel(&self) -> RemoteChannel {
//         panic!()
//     }

//     pub fn push_local_event(&self, event:E) {
//         panic!()
//     }
// }

// pub struct RemoteChannel {}
// impl RemoteChannel {
//     pub fn peer_addr(&self) -> SocketAddr {
//         panic!()
//     }
//     pub fn cast<M, A>(&self, _indication: Indication<M, A>) {}
// }
