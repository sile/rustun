use Method;
use method;
use types::U12;

pub const METHOD_BINDING: u16 = 0x0001;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Binding;
impl Method for Binding {
    fn from_u12(value: U12) -> Option<Self> {
        if value.as_u16() == METHOD_BINDING {
            Some(Binding)
        } else {
            None
        }
    }
    fn as_u12(&self) -> U12 {
        U12::from_u16(METHOD_BINDING).unwrap()
    }
}
impl method::Requestable for Binding {}
impl method::Indicatable for Binding {}
