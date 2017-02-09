use std::mem;
use futures::{Future, Poll, Sink, AsyncSink, Async, Stream};

use {Error, StunMethod, ErrorKind};
use attribute::Attribute;
use message::{Message, Class, TransactionId};
use transport::TransportChannel;

// TODO: move to builtin module
#[derive(Debug)]
pub struct Client<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    tx: C::Sender,
    rx: C::Receiver,
}
impl<M, A, C> Client<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    pub fn new(channel: C) -> Self {
        let (tx, rx) = channel.channel();
        Client { tx: tx, rx: rx }
    }
    pub fn call(mut self, message: Message<M, A>) -> Call<M, A, C> {
        if let Err(e) = message.class().expect(Class::Request) {
            Call::failed(e)
        } else {
            let transaction_id = message.transaction_id();
            match self.tx.start_send(message) {
                Err(e) => Call::failed(e),
                Ok(AsyncSink::NotReady(_)) => Call::failed(ErrorKind::ChannelFull.into()),
                Ok(AsyncSink::Ready) => Call::new(self, transaction_id),
            }
        }
    }
    pub fn cast(mut self, message: Message<M, A>) -> Cast<M, A, C> {
        if let Err(e) = message.class().expect(Class::Indication) {
            Cast::failed(e)
        } else {
            match self.tx.start_send(message) {
                Err(e) => Cast::failed(e),
                Ok(AsyncSink::NotReady(_)) => Cast::failed(ErrorKind::ChannelFull.into()),
                Ok(AsyncSink::Ready) => Cast::new(self),
            }
        }
    }
}

pub struct Cast<M, A, C>(CastInner<M, A, C>)
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>;
impl<M, A, C> Cast<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    fn new(client: Client<M, A, C>) -> Self {
        Cast(CastInner::Send(client))
    }
    fn failed(error: Error) -> Self {
        Cast(CastInner::Failed(error))
    }
}

pub enum CastInner<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    Failed(Error),
    Send(Client<M, A, C>),
    Polled,
}
impl<M, A, C> Future for CastInner<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    type Item = Client<M, A, C>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, CastInner::Polled) {
            CastInner::Failed(e) => Err(e),
            CastInner::Send(mut c) => {
                if c.tx.poll_complete()?.is_ready() {
                    Ok(Async::Ready(c))
                } else {
                    *self = CastInner::Send(c);
                    Ok(Async::NotReady)
                }
            }
            CastInner::Polled => panic!("Cannot poll CastInner twice"),
        }
    }
}

pub struct Call<M, A, C>(CallInner<M, A, C>)
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>;
impl<M, A, C> Call<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    fn new(client: Client<M, A, C>, transaction_id: TransactionId) -> Self {
        Call(CallInner::Send(client, transaction_id))
    }
    fn failed(error: Error) -> Self {
        Call(CallInner::Failed(error))
    }
}
impl<M, A, C> Future for Call<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    type Item = (Client<M, A, C>, Message<M, A>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}


pub enum CallInner<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    Failed(Error),
    Send(Client<M, A, C>, TransactionId),
    Recv(Client<M, A, C>, TransactionId),
    Polled,
}
impl<M, A, C> Future for CallInner<M, A, C>
    where M: StunMethod,
          A: Attribute,
          C: TransportChannel<M, A>
{
    type Item = (Client<M, A, C>, Message<M, A>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, CallInner::Polled) {
            CallInner::Failed(e) => Err(e),
            CallInner::Send(mut c, transaction_id) => {
                if c.tx.poll_complete()?.is_ready() {
                    *self = CallInner::Recv(c, transaction_id);
                    self.poll()
                } else {
                    *self = CallInner::Send(c, transaction_id);
                    Ok(Async::NotReady)
                }
            }
            CallInner::Recv(mut c, transaction_id) => {
                match c.rx.poll()? {
                    Async::NotReady => {
                        *self = CallInner::Recv(c, transaction_id);
                        Ok(Async::NotReady)
                    }
                    Async::Ready(None) => Err(ErrorKind::ChannelDisconnected.into()),
                    Async::Ready(Some(m)) => {
                        let m = m?;
                        if m.transaction_id() != transaction_id {
                            *self = CallInner::Recv(c, transaction_id);
                            self.poll()
                        } else if !m.class().is_response() {
                            Err(ErrorKind::NotResponse(m.class()).into())
                        } else {
                            Ok(Async::Ready((c, m)))
                        }
                    }
                }
            }
            CallInner::Polled => panic!("Cannot poll CallInner twice"),
        }
    }
}
