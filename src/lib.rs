extern crate rand;
extern crate fibers;
extern crate futures;
extern crate byteorder;
#[macro_use]
extern crate error_chain;

pub mod client;
pub mod server;

pub const DEFAULT_PORT: u16 = 3478;
pub const DEFAULT_TLS_PORT: u16 = 5349;

pub const MAGIC_COOKIE: u32 = 0x2112A442;

pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 568;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct U12(u16);
impl U12 {
    pub fn from_u16(value: u16) -> Option<Self> {
        if value < 0x1000 {
            Some(U12(value))
        } else {
            None
        }
    }
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MessageType {
    pub class: MessageClass,
    pub method: Method,
}
impl MessageType {
    /// TODO:
    ///
    /// ```text
    /// 0                 1
    /// 2  3  4 5 6 7 8 9 0 1 2 3 4 5
    ///
    /// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |M |M |M|M|M|C|M|M|M|C|M|M|M|M|
    /// |11|10|9|8|7|1|6|5|4|0|3|2|1|0|
    /// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
    ///
    /// Figure 3: Format of STUN Message Type Field
    /// ```
    pub fn as_u16(&self) -> u16 {
        let class = self.class.as_u8() as u16;
        let method = self.method.as_u12().as_u16();
        ((method & 0b0000_0000_1111) << 0) | ((class & 0b01) << 4) |
        ((method & 0b0000_0111_0000) << 5) | ((class & 0b10) << 7) |
        ((method & 0b1111_1000_0000) << 9)
    }

    pub fn from_u16(value: u16) -> Self {
        let class = ((value >> 4) & 0b01) | ((value >> 7) & 0b10);
        let class = MessageClass::from_u8(class as u8).unwrap();

        let method = (value & 0b0000_0000_1111) | ((value >> 1) & 0b0000_0111_0000) |
                     ((value >> 2) & 0b1111_1000_0000);
        let method = Method::from_u12(U12::from_u16(method).unwrap());
        MessageType {
            class: class,
            method: method,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MessageClass {
    Request,
    SuccessResponse,
    FailureResponse,
    Indication,
}
impl MessageClass {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(MessageClass::Request),
            0b01 => Some(MessageClass::Indication),
            0b10 => Some(MessageClass::SuccessResponse),
            0b11 => Some(MessageClass::FailureResponse),
            _ => None,
        }
    }
    pub fn as_u8(&self) -> u8 {
        match *self {
            MessageClass::Request => 0b00,
            MessageClass::Indication => 0b01,
            MessageClass::SuccessResponse => 0b10,
            MessageClass::FailureResponse => 0b11,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Method {
    Binding,
    Other(U12),
}
impl Method {
    pub fn from_u12(value: U12) -> Self {
        match value.as_u16() {
            0x001 => Method::Binding,
            _ => Method::Other(value),
        }
    }
    pub fn as_u12(&self) -> U12 {
        match *self {
            Method::Binding => U12::from_u16(0x001).unwrap(),
            Method::Other(value) => value,
        }
    }
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
    foreign_links {
        Io(std::io::Error);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
