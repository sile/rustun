use Attribute;
use types::U12;
use message::{Class, Request, Response};
use message2;

pub trait Method: Sized {
    fn from_u12(value: U12) -> Option<Self>;
    fn as_u12(&self) -> U12;
    fn permits_class(&self, class: Class) -> bool;

    fn request<A: Attribute>(self) -> Request<Self, A> {
        Request::new(self)
    }
    fn success_response<A: Attribute>(self) -> Response<Self, A> {
        Response::new_success(self)
    }
    // fn indication<A: Attribute>(self) -> Indication<Self, A> {
    //     Indication::new(self)
    // }
}
impl Method for U12 {
    fn from_u12(value: U12) -> Option<Self> {
        Some(value)
    }
    fn as_u12(&self) -> U12 {
        *self
    }
    fn permits_class(&self, _class: Class) -> bool {
        true
    }
}

pub trait Requestable: Method + Sized {
    fn request<M, A>(self) -> message2::Request<M, A>
        where M: From<Self> + Method,
              A: Attribute
    {
        message2::Request::new(self)
    }
}

pub trait Indicatable: Method + Sized {
    fn indication<M, A>(self) -> message2::Indication<M, A>
        where M: From<Self> + Method,
              A: Attribute
    {
        message2::Indication::new(self)
    }
}
