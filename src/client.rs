use futures::{Future, BoxFuture};

use {Method, Attribute, Error};
use message::{Indication, Request, Response};

pub trait Client<M, A>
    where M: Method,
          A: Attribute
{
    type Call: Future<Item = Response<M, A>, Error = Error>;
    type Cast: Future<Item = (), Error = Error>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call;
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast;
    fn boxed(mut self) -> BoxClient<M, A>
        where Self: Sized + Send + 'static,
              Self::Call: Send + 'static,
              Self::Cast: Send + 'static
    {
        let f = move |message| match message {
            Ok(request) => Ok(self.call(request).boxed()),
            Err(indication) => Err(self.cast(indication).boxed()),
        };
        BoxClient(Box::new(f))
    }
}

type BoxClientFn<M, A> = Box<FnMut(Result<Request<M, A>, Indication<M, A>>)
                                   -> Result<BoxFuture<Response<M, A>, Error>,
                                              BoxFuture<(), Error>> + Send + 'static>;

pub struct BoxClient<M, A>(BoxClientFn<M, A>);
impl<M, A> Client<M, A> for BoxClient<M, A>
    where M: Method,
          A: Attribute
{
    type Call = BoxFuture<Response<M, A>, Error>;
    type Cast = BoxFuture<(), Error>;
    fn call(&mut self, message: Request<M, A>) -> Self::Call {
        (self.0)(Ok(message)).ok().unwrap()
    }
    fn cast(&mut self, message: Indication<M, A>) -> Self::Cast {
        (self.0)(Err(message)).err().unwrap()
    }
}
