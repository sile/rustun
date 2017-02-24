use Attribute;
use types::U12;
use message::{Request, Indication};

pub trait Method: Sized {
    fn from_u12(value: U12) -> Option<Self>;
    fn as_u12(&self) -> U12;
}
impl Method for U12 {
    fn from_u12(value: U12) -> Option<Self> {
        Some(value)
    }
    fn as_u12(&self) -> U12 {
        *self
    }
}

pub trait Requestable: Method + Sized {
    fn request<M, A>(self) -> Request<M, A>
        where M: From<Self> + Method,
              A: Attribute
    {
        Request::new(self)
    }
}

pub trait Indicatable: Method + Sized {
    fn indication<M, A>(self) -> Indication<M, A>
        where M: From<Self> + Method,
              A: Attribute
    {
        Indication::new(self)
    }
}
