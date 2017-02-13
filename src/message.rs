use std::io::{Read, Write, Cursor};
use rand;

use {Result, Error, Method, Attribute};
use types::{U12, TransactionId};
use attribute::RawAttribute;
use io::{ReadExt, WriteExt};
use constants::MAGIC_COOKIE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type<M> {
    pub class: Class,
    pub method: M,
}
impl<M: Method> Type<M> {
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
        fail_if!(value >> 14 != 0,
                 Error::NotStunMessage("First two-bits of STUN message must be 0".to_string()))?;
        let class = ((value >> 4) & 0b01) | ((value >> 7) & 0b10);
        let class = Class::from_u8(class as u8).unwrap();

        let method = (value & 0b0000_0000_1111) | ((value >> 1) & 0b0000_0111_0000) |
                     ((value >> 2) & 0b1111_1000_0000);
        let method = U12::from_u16(method).unwrap();
        let method = M::from_u12(method).ok_or(Error::UnknownMethod(method))?;
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
    ErrorResponse = 0b10,
    Indication = 0b11,
}
impl Class {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Class::Request),
            0b01 => Some(Class::Indication),
            0b10 => Some(Class::SuccessResponse),
            0b11 => Some(Class::ErrorResponse),
            _ => None,
        }
    }
    pub fn is_request(&self) -> bool {
        Class::Request == *self
    }
    pub fn is_indication(&self) -> bool {
        Class::Indication == *self
    }
    pub fn is_response(&self) -> bool {
        match *self {
            Class::SuccessResponse | Class::ErrorResponse => true,
            _ => false,
        }
    }
    pub fn is_success_response(&self) -> bool {
        Class::SuccessResponse == *self
    }
    pub fn is_error_response(&self) -> bool {
        Class::ErrorResponse == *self
    }
}

pub type RawMessage = Message<U12, RawAttribute>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<M, A> {
    message_type: Type<M>,
    transaction_id: TransactionId,
    attributes: Vec<A>,
}
impl<M, A> Message<M, A>
    where M: Method,
          A: Attribute
{
    pub fn new(message_type: Type<M>) -> Self {
        Message {
            message_type: message_type,
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }
    pub fn class(&self) -> Class {
        self.message_type.class
    }
    pub fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }

    pub fn request(method: M) -> Self {
        Self::new(Type {
            class: Class::Request,
            method: method,
        })
    }
    pub fn indication(method: M) -> Self {
        Self::new(Type {
            class: Class::Indication,
            method: method,
        })
    }
    pub fn success_response(self) -> Self {
        Message {
            message_type: Type {
                class: Class::SuccessResponse,
                method: self.message_type.method,
            },
            transaction_id: self.transaction_id,
            attributes: Vec::new(),
        }
    }
    pub fn failure_response(self) -> Self {
        Message {
            message_type: Type {
                class: Class::ErrorResponse,
                method: self.message_type.method,
            },
            transaction_id: self.transaction_id,
            attributes: Vec::new(),
        }
    }
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) {
        self.attributes.push(attribute.into());
    }
}
impl RawMessage {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let message_type = may_fail!(reader.read_u16())?;
        let message_type = may_fail!(Type::from_u16(message_type))?;
        let message_len = may_fail!(reader.read_u16())?;
        fail_if!(message_len % 4 != 0,
                 Error::NotStunMessage(format!("Unexpected message length: {} % 4 != 0",
                                               message_len)))?;
        let magic_cookie = may_fail!(reader.read_u32())?;
        fail_if!(magic_cookie != MAGIC_COOKIE,
                 Error::NotStunMessage(format!("Unexpected magic cookie: actual={}, \
                                                expected={}",
                                               magic_cookie,
                                               MAGIC_COOKIE)))?;

        let mut transaction_id: [u8; 12] = [0; 12];
        may_fail!(reader.read_exact_bytes(&mut transaction_id))?;

        let mut attrs = Vec::new();
        let mut reader = reader.take(message_len as u64);
        while reader.limit() > 0 {
            let attr = may_fail!(RawAttribute::read_from(&mut reader))?;
            attrs.push(attr);
        }
        Ok(Message {
            message_type: message_type,
            transaction_id: transaction_id,
            attributes: attrs,
        })
    }
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut temp_writer = Cursor::new(vec![0; 20]);
        may_fail!(temp_writer.write_u16(self.message_type.as_u16()))?;
        may_fail!(temp_writer.write_u16(0))?; // dummy length
        may_fail!(temp_writer.write_u32(MAGIC_COOKIE))?;
        may_fail!(temp_writer.write_all_bytes(&self.transaction_id))?;
        for attr in self.attributes.iter() {
            attr.write_to(&mut temp_writer)?;
        }
        let attrs_len = temp_writer.get_ref().len() - 20;
        fail_if!(attrs_len >= 0x10000,
                 "Too large message length: actual={}, limit=0xFFFF",
                 attrs_len)?;
        temp_writer.set_position(2);
        may_fail!(temp_writer.write_u16(attrs_len as u16))?;

        let buf = temp_writer.into_inner();
        may_fail!(writer.write_all_bytes(&buf))?;
        Ok(())
    }
}
