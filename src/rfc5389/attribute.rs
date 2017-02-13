use std::io::{Read, Write};

use {Result, Error, AttributeType};

use super::constants;
use super::attributes;

macro_rules! impl_from {
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
impl_from!(MappedAddress);
impl_from!(Username);
impl_from!(MessageIntegrity);
impl_from!(ErrorCode);
impl_from!(UnknownAttributes);
impl_from!(Realm);
impl_from!(Nonce);
impl_from!(XorMappedAddress);
impl_from!(Software);
impl_from!(AlternateServer);
impl_from!(Fingerprint);
impl ::Attribute for Attribute {
    fn get_type(&self) -> AttributeType {
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
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        match attr_type.as_u16() {
            constants::ATTR_TYPE_MAPPED_ADDRESS => {
                attributes::MappedAddress::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_USERNAME => {
                attributes::Username::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_MESSAGE_INTEGRITY => {
                attributes::MessageIntegrity::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_ERROR_CODE => {
                attributes::ErrorCode::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_UNKNOWN_ATTRIBUTES => {
                attributes::UnknownAttributes::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_REALM => {
                attributes::Realm::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_NONCE => {
                attributes::Nonce::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_XOR_MAPPED_ADDRESS => {
                attributes::XorMappedAddress::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_SOFTWARE => {
                attributes::Software::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_ALTERNATE_SERVER => {
                attributes::AlternateServer::read_value_from(attr_type, reader).map(From::from)
            }
            constants::ATTR_TYPE_FINGERPRINT => {
                attributes::Fingerprint::read_value_from(attr_type, reader).map(From::from)
            }
            _ => Err(Error::UnknownAttribute(attr_type.as_u16())),
        }
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        match *self {
            Attribute::MappedAddress(ref a) => a.write_value_to(writer),
            Attribute::Username(ref a) => a.write_value_to(writer),
            Attribute::MessageIntegrity(ref a) => a.write_value_to(writer),
            Attribute::ErrorCode(ref a) => a.write_value_to(writer),
            Attribute::UnknownAttributes(ref a) => a.write_value_to(writer),
            Attribute::Realm(ref a) => a.write_value_to(writer),
            Attribute::Nonce(ref a) => a.write_value_to(writer),
            Attribute::XorMappedAddress(ref a) => a.write_value_to(writer),
            Attribute::Software(ref a) => a.write_value_to(writer),
            Attribute::AlternateServer(ref a) => a.write_value_to(writer),
            Attribute::Fingerprint(ref a) => a.write_value_to(writer),
        }
    }
}
