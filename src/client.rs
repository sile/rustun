use std::marker::PhantomData;
use futures::{self, Future, Poll, Async};
use futures::future::{Either, Failed};

use {Method, Attribute, Error};
use message::{Indication, Request, Response, RawMessage};

pub trait Client {
    type CallRaw: Future<Item = RawMessage, Error = Error>;
    type CastRaw: Future<Item = (), Error = Error>;
    fn call<M, A>(&mut self, message: Request<M, A>) -> Call<M, A, Self::CallRaw>
        where M: Method,
              A: Attribute
    {
        match track_err!(RawMessage::try_from_request(message)) {
            Err(e) => Call(Either::A(futures::failed(e)), PhantomData),
            Ok(m) => Call(Either::B(self.call_raw(m)), PhantomData),
        }
    }
    fn cast<M, A>(&mut self, message: Indication<M, A>) -> Cast<Self::CastRaw>
        where M: Method,
              A: Attribute
    {
        match track_err!(RawMessage::try_from_indication(message)) {
            Err(e) => Cast(Either::A(futures::failed(e))),
            Ok(m) => Cast(Either::B(self.cast_raw(m))),
        }
    }

    fn call_raw(&mut self, message: RawMessage) -> Self::CallRaw;
    fn cast_raw(&mut self, message: RawMessage) -> Self::CastRaw;
}

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
