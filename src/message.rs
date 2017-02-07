use {StunMethod, Result, ErrorKind};
use types::U12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type<M> {
    pub class: Class,
    pub method: M,
}
impl<M: StunMethod> Type<M> {
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
        let class = self.class as u16;
        let method = self.method.as_u12().as_u16();
        ((method & 0b0000_0000_1111) << 0) | ((class & 0b01) << 4) |
        ((method & 0b0000_0111_0000) << 5) | ((class & 0b10) << 7) |
        ((method & 0b1111_1000_0000) << 9)
    }

    pub fn from_u16(value: u16) -> Result<Self> {
        let class = ((value >> 4) & 0b01) | ((value >> 7) & 0b10);
        let class = Class::from_u8(class as u8).unwrap();

        let method = (value & 0b0000_0000_1111) | ((value >> 1) & 0b0000_0111_0000) |
                     ((value >> 2) & 0b1111_1000_0000);
        let method = U12::from_u16(method).unwrap();
        let method = M::from_u12(method).ok_or(ErrorKind::UnknownMethod(method))?;
        Ok(Type {
            class: class,
            method: method,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Class {
    Request = 0b00,
    SuccessResponse = 0b01,
    FailureResponse = 0b10,
    Indication = 0b11,
}
impl Class {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Class::Request),
            0b01 => Some(Class::Indication),
            0b10 => Some(Class::SuccessResponse),
            0b11 => Some(Class::FailureResponse),
            _ => None,
        }
    }
}
