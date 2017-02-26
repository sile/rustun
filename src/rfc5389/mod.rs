use trackable::error::ErrorKindExt;

use {Result, ErrorKind};
use message::{self, RawMessage};
use attribute::{self, RawAttribute};
use types::U12;
use client;

pub mod methods;
pub mod attributes;
pub mod handlers;

pub type UdpClient = client::UdpClient;
// pub type TcpClient = clients::TcpClient;

pub type Request = message::Request<Method, Attribute>;
pub type Response = message::Response<Method, Attribute>;
pub type Indication = message::Indication<Method, Attribute>;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Method {
    Binding(methods::Binding),
}
impl Method {
    pub fn binding() -> Self {
        Method::Binding(methods::Binding)
    }
}
impl ::Method for Method {
    fn from_u12(value: U12) -> Option<Self> {
        match value.as_u16() {
            methods::METHOD_BINDING => Some(Method::Binding(methods::Binding)),
            _ => None,
        }
    }
    fn as_u12(&self) -> U12 {
        match *self {
            Method::Binding(ref m) => m.as_u12(),
        }
    }
}
impl From<methods::Binding> for Method {
    fn from(f: methods::Binding) -> Self {
        Method::Binding(f)
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
