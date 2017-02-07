extern crate rand;
extern crate fibers;
extern crate futures;
extern crate byteorder;
#[macro_use]
extern crate error_chain;

use types::U12;

pub mod client;
pub mod server;
pub mod message;
pub mod types;
pub mod rfc5389;

pub const DEFAULT_PORT: u16 = 3478;
pub const DEFAULT_TLS_PORT: u16 = 5349;

pub const MAGIC_COOKIE: u32 = 0x2112A442;

pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 568;

// TODO: rename
pub trait StunMethod: Sized {
    fn from_u12(value: U12) -> Option<Self>;
    fn as_u12(&self) -> U12;
}

#[derive(Debug, Clone, Copy)]
pub enum AttrType {
    MappedAddress,
    Username,
    MessageIntegrity,
    ErrorCode,
    UnknownAttributes,
    Realm,
    Nonce,
    XorMappedAddress,
    Software,
    AlternateServer,
    Fingerprint,
    Other(u16),
}
impl AttrType {
    pub fn from_u16(value: u16) -> Self {
        match value {
            0x0001 => AttrType::MappedAddress,
            0x0006 => AttrType::Username,
            0x0008 => AttrType::MessageIntegrity,
            0x0009 => AttrType::ErrorCode,
            0x000A => AttrType::UnknownAttributes,
            0x0014 => AttrType::Realm,
            0x0015 => AttrType::Nonce,
            0x0020 => AttrType::XorMappedAddress,
            0x8022 => AttrType::Software,
            0x8023 => AttrType::AlternateServer,
            0x8028 => AttrType::Fingerprint,
            other => AttrType::Other(other),
        }
    }
    pub fn as_u16(&self) -> u16 {
        match *self {
            AttrType::MappedAddress => 0x0001,
            AttrType::Username => 0x0006,
            AttrType::MessageIntegrity => 0x0008,
            AttrType::ErrorCode => 0x0009,
            AttrType::UnknownAttributes => 0x000A,
            AttrType::Realm => 0x0014,
            AttrType::Nonce => 0x0015,
            AttrType::XorMappedAddress => 0x0020,
            AttrType::Software => 0x0022,
            AttrType::AlternateServer => 0x0023,
            AttrType::Fingerprint => 0x8028,
            AttrType::Other(value) => value,
        }
    }
    pub fn is_comprehension_required(&self) -> bool {
        self.as_u16() < 0x8000
    }
}

error_chain!{
    errors {
        UnknownMethod(method: U12)
    }
    foreign_links {
        Io(std::io::Error);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
