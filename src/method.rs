use Attribute;
use types::U12;
use message::{Class, Request};

pub trait Method: Sized {
    fn from_u12(value: U12) -> Option<Self>;
    fn as_u12(&self) -> U12;
    fn permits_class(&self, class: Class) -> bool;

    fn request<A: Attribute>(self) -> Request<Self, A> {
        Request::new(self)
    }
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
