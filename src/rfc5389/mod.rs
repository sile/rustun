use StunMethod;
use types::U12;
use message;

pub use self::attribute::Attribute;

pub mod attributes;

pub mod constants {
    pub const ATTR_TYPE_MAPPED_ADDRESS: u16 = 0x0001;
    pub const ATTR_TYPE_USERNAME: u16 = 0x0006;
    pub const ATTR_TYPE_MESSAGE_INTEGRITY: u16 = 0x0008;
    pub const ATTR_TYPE_ERROR_CODE: u16 = 0x0009;
    pub const ATTR_TYPE_UNKNOWN_ATTRIBUTES: u16 = 0x000A;
    pub const ATTR_TYPE_REALM: u16 = 0x0014;
    pub const ATTR_TYPE_NONCE: u16 = 0x0015;
    pub const ATTR_TYPE_XOR_MAPPED_ADDRESS: u16 = 0x0020;
    pub const ATTR_TYPE_SOFTWARE: u16 = 0x8022;
    pub const ATTR_TYPE_ALTERNATE_SERVER: u16 = 0x8023;
    pub const ATTR_TYPE_FINGERPRINT: u16 = 0x8028;
}

mod attribute;

pub type Message = message::Message<Method, Attribute>;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Method {
    Binding = 0x001,
}
impl StunMethod for Method {
    fn from_u12(value: U12) -> Option<Self> {
        match value.as_u16() {
            0x001 => Some(Method::Binding),
            _ => None,
        }
    }
    fn as_u12(&self) -> U12 {
        U12::from_u8(*self as u8)
    }
    fn permits_class(&self, _class: message::Class) -> bool {
        // TODO
        true
    }
}
