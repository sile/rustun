use std::marker::PhantomData;
use std::time::{SystemTime, Duration};
use std::sync::Arc;
use std::sync::atomic;
use fibers::time::timer::{self, Timeout};
use futures::{Future, Poll, Async};
use trackable::error::ErrorKindExt;

use {Method, Attribute, Client, ErrorKind, Error};
use message::{Indication, Request};
use constants;

#[derive(Debug)]
pub struct RateLimitedClient<C, M, A> {
    inner: C,
    max_concurrency: usize,
    min_interval: Duration,
    concurrency: Arc<atomic::AtomicUsize>,
    wait_until: SystemTime,
    _phantom: PhantomData<(M, A)>,
}
impl<C, M, A> RateLimitedClient<C, M, A>
    where C: Client<M, A>,
          M: Method,
          A: Attribute
{
    pub fn new(inner: C) -> Self {
        RateLimitedClient {
            inner: inner,
            max_concurrency: constants::DEFAULT_MAX_CLIENT_CONCURRENCY,
            min_interval: Duration::from_millis(constants::DEFAULT_MIN_TRANSACTION_INTERVAL_MS),
            concurrency: Arc::new(atomic::AtomicUsize::new(0)),
            wait_until: SystemTime::now(),
            _phantom: PhantomData,
        }
    }
    pub fn set_max_concurrency(&mut self, max: usize) -> &mut Self {
        self.max_concurrency = max;
        self
    }
    pub fn set_min_interval(&mut self, min: Duration) -> &mut Self {
        self.min_interval = min;
        self
    }
    pub fn into_inner(self) -> C {
        self.inner
    }
}
impl<C, M, A> Client<M, A> for RateLimitedClient<C, M, A>
    where C: Client<M, A>,
          M: Method,
          A: Attribute
{
    type Call = RateLimited<C::Call>;
    type Cast = RateLimited<C::Cast>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call {
        let future = self.inner.call(message);
        RateLimited::new(future, self)
    }
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast {
        let future = self.inner.cast(message);
        RateLimited::new(future, self)
    }
}

pub struct RateLimited<F> {
    future: F,
    is_full: bool,
    wait_until: Option<Timeout>,
    concurrency: Arc<atomic::AtomicUsize>,
}
impl<F> RateLimited<F> {
    fn new<C, M, A>(future: F, client: &mut RateLimitedClient<C, M, A>) -> Self {
        let current = client.concurrency.fetch_add(1, atomic::Ordering::SeqCst);
        let is_full = current >= client.max_concurrency;
        let wait_until = if is_full {
            None
        } else {
            let prev_wait_until = client.wait_until;
            let now = SystemTime::now();
            client.wait_until = now + client.min_interval;
            if let Ok(duration) = prev_wait_until.duration_since(now) {
                Some(timer::timeout(duration))
            } else {
                None
            }
        };
        RateLimited {
            future: future,
            is_full: is_full,
            wait_until: wait_until,
            concurrency: client.concurrency.clone(),
        }
    }
}
impl<F> Drop for RateLimited<F> {
    fn drop(&mut self) {
        self.concurrency.fetch_sub(1, atomic::Ordering::SeqCst);
    }
}
impl<F> Future for RateLimited<F>
    where F: Future<Error = Error>
{
    type Item = F::Item;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.is_full {
            return Err(ErrorKind::Full.into());
        }
        if let Some(ref mut f) = self.wait_until {
            if let Async::NotReady = track_try!(f.poll()
                .map_err(|_| ErrorKind::Failed.cause("Timeout object aborted"))) {
                return Ok(Async::NotReady);
            }
        }
        self.wait_until = None;

        track_err!(self.future.poll())
    }
}
