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
    /// See [methods::Binding](methods/struct.Binding.html).
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
impl From<methods::Binding> for Method {
    fn from(_: methods::Binding) -> Self {
        Method::Binding
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

/// Attribute set that are defined in [RFC 5389](https://tools.ietf.org/html/rfc5389).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Attribute {
    MappedAddress(attributes::MappedAddress),
    Username(attributes::Username),
    MessageIntegrity(attributes::MessageIntegrity),
    ErrorCode(attributes::ErrorCode),
    UnknownAttributes(attributes::UnknownAttributes),
    Realm(attributes::Realm),
    Nonce(attributes::Nonce),
    XorMappedAddress(attributes::XorMappedAddress),
    Software(attributes::Software),
    AlternateServer(attributes::AlternateServer),
    Fingerprint(attributes::Fingerprint),
}
impl_attr_from!(MappedAddress);
impl_attr_from!(Username);
impl_attr_from!(MessageIntegrity);
impl_attr_from!(ErrorCode);
impl_attr_from!(UnknownAttributes);
impl_attr_from!(Realm);
impl_attr_from!(Nonce);
impl_attr_from!(XorMappedAddress);
impl_attr_from!(Software);
impl_attr_from!(AlternateServer);
impl_attr_from!(Fingerprint);
impl ::Attribute for Attribute {
    fn get_type(&self) -> attribute::Type {
        match *self {
            Attribute::MappedAddress(ref a) => a.get_type(),
            Attribute::Username(ref a) => a.get_type(),
            Attribute::MessageIntegrity(ref a) => a.get_type(),
            Attribute::ErrorCode(ref a) => a.get_type(),
            Attribute::UnknownAttributes(ref a) => a.get_type(),
            Attribute::Realm(ref a) => a.get_type(),
            Attribute::Nonce(ref a) => a.get_type(),
            Attribute::XorMappedAddress(ref a) => a.get_type(),
            Attribute::Software(ref a) => a.get_type(),
            Attribute::AlternateServer(ref a) => a.get_type(),
            Attribute::Fingerprint(ref a) => a.get_type(),
        }
    }
    fn try_from_raw(attr: &RawAttribute, message: &RawMessage) -> Result<Self> {
        match attr.get_type().as_u16() {
            attributes::TYPE_MAPPED_ADDRESS => {
                attributes::MappedAddress::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_USERNAME => {
                attributes::Username::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_MESSAGE_INTEGRITY => {
                attributes::MessageIntegrity::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_ERROR_CODE => {
                attributes::ErrorCode::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_UNKNOWN_ATTRIBUTES => {
                attributes::UnknownAttributes::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_REALM => {
                attributes::Realm::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_NONCE => {
                attributes::Nonce::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_XOR_MAPPED_ADDRESS => {
                attributes::XorMappedAddress::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_SOFTWARE => {
                attributes::Software::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_ALTERNATE_SERVER => {
                attributes::AlternateServer::try_from_raw(attr, message).map(From::from)
            }
            attributes::TYPE_FINGERPRINT => {
                attributes::Fingerprint::try_from_raw(attr, message).map(From::from)
            }
            t => Err(ErrorKind::Unsupported.cause(format!("Unknown attribute: type={}", t))),
        }
    }
    fn encode_value(&self, message: &RawMessage) -> Result<Vec<u8>> {
        match *self {
            Attribute::MappedAddress(ref a) => a.encode_value(message),
            Attribute::Username(ref a) => a.encode_value(message),
            Attribute::MessageIntegrity(ref a) => a.encode_value(message),
            Attribute::ErrorCode(ref a) => a.encode_value(message),
            Attribute::UnknownAttributes(ref a) => a.encode_value(message),
            Attribute::Realm(ref a) => a.encode_value(message),
            Attribute::Nonce(ref a) => a.encode_value(message),
            Attribute::XorMappedAddress(ref a) => a.encode_value(message),
            Attribute::Software(ref a) => a.encode_value(message),
            Attribute::AlternateServer(ref a) => a.encode_value(message),
            Attribute::Fingerprint(ref a) => a.encode_value(message),
        }
    }
}
