//! STUN client related components.
use futures::{failed, Future};
use futures::future::Either;

use {Method, Attribute, Error};
use message::{Indication, Request, RawMessage};

pub use self::base::BaseClient;
pub use self::udp::UdpClient;
pub use self::tcp::TcpClient;

pub mod futures {
    //! `Future` trait implementations.
    pub use super::futures_impl::{Call, Cast};
    pub use super::base::{BaseCallRaw, BaseCastRaw};
    pub use super::udp::{UdpCallRaw, UdpCastRaw};
    pub use super::tcp::{TcpCallRaw, TcpCastRaw, InitTcpClient};
}

mod base;
mod udp;
mod tcp;

/// STUN client.
pub trait Client {
    /// `Future` type to handle a request/response transaction using `RawMessage`.
    type CallRaw: Future<Item = RawMessage, Error = Error>;

    /// `Future` type to handle a indication transaction using `RawMessage`.
    type CastRaw: Future<Item = (), Error = Error>;

    /// Makes a `Future` that sends the request message to a server and
    /// waits the response from it.
    fn call<M, A>(&mut self, message: Request<M, A>) -> futures::Call<M, A, Self::CallRaw>
        where M: Method,
              A: Attribute
    {
        match track_err!(RawMessage::try_from_request(message)) {
            Err(e) => futures_impl::call(Either::A(failed(e))),
            Ok(m) => futures_impl::call(Either::B(self.call_raw(m))),
        }
    }

    /// Makes a `Future` that sends the indication message to a server.
    fn cast<M, A>(&mut self, message: Indication<M, A>) -> futures::Cast<Self::CastRaw>
        where M: Method,
              A: Attribute
    {
        match track_err!(RawMessage::try_from_indication(message)) {
            Err(e) => futures_impl::cast(Either::A(failed(e))),
            Ok(m) => futures_impl::cast(Either::B(self.cast_raw(m))),
        }
    }

    /// Makes a `Future` that sends the raw request message to a server and
    /// waits the response from it.
    fn call_raw(&mut self, message: RawMessage) -> Self::CallRaw;

    /// Makes a `Future` that sends the raw indication message to a server.
    fn cast_raw(&mut self, message: RawMessage) -> Self::CastRaw;
}

mod futures_impl {
    use std::fmt;
    use std::marker::PhantomData;
    use futures::{Future, Poll, Async};
    use futures::future::{Either, Failed};

    use {Method, Attribute, Error};
    use message::{Response, RawMessage};

    pub fn cast<F>(future: Either<Failed<(), Error>, F>) -> Cast<F> {
        Cast(future)
    }

    /// `Future` that handle a indication transaction.
    ///
    /// This is created by calling `Client::cast` method.
    pub struct Cast<F>(Either<Failed<(), Error>, F>);
    impl<F> Future for Cast<F>
        where F: Future<Item = (), Error = Error>
    {
        type Item = ();
        type Error = Error;
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            track_err!(self.0.poll())
        }
    }
    impl<F> fmt::Debug for Cast<F> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self.0 {
                Either::A(_) => write!(f, "Cast(A(_))"),
                Either::B(_) => write!(f, "Cast(B(_))"),
            }
        }
    }

    pub fn call<M, A, F>(future: Either<Failed<RawMessage, Error>, F>) -> Call<M, A, F> {
        Call(future, PhantomData)
    }

    /// `Future` that handle a request/response transaction.
    ///
    /// This is created by calling `Client::call` method.
    pub struct Call<M, A, F>(Either<Failed<RawMessage, Error>, F>, PhantomData<(M, A)>);
    impl<M, A, F> Future for Call<M, A, F>
        where M: Method,
              A: Attribute,
              F: Future<Item = RawMessage, Error = Error>
    {
        type Item = Response<M, A>;
        type Error = Error;
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            if let Async::Ready(message) = track_try!(self.0.poll()) {
                track_err!(message.try_into_response()).map(Async::Ready)
            } else {
                Ok(Async::NotReady)
            }
        }
    }
    impl<M, A, F> fmt::Debug for Call<M, A, F> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self.0 {
                Either::A(_) => write!(f, "Call(A(_), _)"),
                Either::B(_) => write!(f, "Call(B(_), _)"),
            }
        }
    }
}
