//! [RFC 5389](https://tools.ietf.org/html/rfc5389) specific components.
use trackable::error::ErrorKindExt;

use {Result, ErrorKind};
use message::RawMessage;
use attribute::{self, RawAttribute};
use types::U12;

pub mod methods;
pub mod attributes;
pub mod handlers;

/// Method set that are defined in [RFC 5389](https://tools.ietf.org/html/rfc5389).
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Method {
    Binding,
}
impl ::Method for Method {
    fn from_u12(value: U12) -> Option<Self> {
        match value.as_u16() {
            methods::METHOD_BINDING => Some(Method::Binding),
            _ => None,
        }
    }
    fn as_u12(&self) -> U12 {
        match *self {
            Method::Binding => methods::Binding.as_u12(),
        }
    }
}

macro_rules! impl_attr_from {
    ($attr:ident) => {
        impl From<attributes::$attr> for Attribute {
            fn from(f: attributes::$attr) -> Self {
                Attribute::$attr(f)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Attribute {
    XorMappedAddress(attributes::XorMappedAddress),
}
impl_attr_from!(XorMappedAddress);
impl ::Attribute for Attribute {
    fn get_type(&self) -> attribute::Type {
        match *self {
            Attribute::XorMappedAddress(ref a) => a.get_type(),
        }
    }
    fn try_from_raw(attr: &RawAttribute, message: &RawMessage) -> Result<Self> {
        match attr.get_type().as_u16() {
            attributes::TYPE_XOR_MAPPED_ADDRESS => {
                attributes::XorMappedAddress::try_from_raw(attr, message).map(From::from)
            }
            t => Err(ErrorKind::Unsupported.cause(format!("Unknown attribute: type={}", t))),
        }
    }
    fn encode_value(&self, message: &RawMessage) -> Result<Vec<u8>> {
        match *self {
            Attribute::XorMappedAddress(ref a) => a.encode_value(message),
        }
    }
}
