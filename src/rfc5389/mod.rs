use StunMethod;
use types::U12;
use message;

pub use self::attribute::Attribute;

pub mod attribute;
pub mod attributes;

pub type Message = message::Message<Method, Attribute>;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Method {
    Reserved = 0x000,
    Binding = 0x001,
    ReservedWasSharedSecret = 0x002,
}
impl StunMethod for Method {
    fn from_u12(value: U12) -> Option<Self> {
        match value.as_u16() {
            0x000 => Some(Method::Reserved),
            0x001 => Some(Method::Binding),
            0x002 => Some(Method::ReservedWasSharedSecret),
            _ => None,
        }
    }
    fn as_u12(&self) -> U12 {
        U12::from_u8(*self as u8)
    }
}
