use std::io::{Read, Write, Cursor};
use rand;
use handy_async::sync_io::{ReadExt, WriteExt};
use track_err::ErrorKindExt;

use {Result, Error, Method, Attribute, ErrorKind};
use types::{U12, TransactionId};
use attribute::RawAttribute;
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
                 ErrorKind::NotStunMessage.cause("First two-bits of STUN message must be 0"))?;
        let class = ((value >> 4) & 0b01) | ((value >> 7) & 0b10);
        let class = Class::from_u8(class as u8).unwrap();
        let method = (value & 0b0000_0000_1111) | ((value >> 1) & 0b0000_0111_0000) |
                     ((value >> 2) & 0b1111_1000_0000);
        let method = U12::from_u16(method).unwrap();
        if let Some(method) = M::from_u12(method) {
            Ok(Type {
                class: class,
                method: method,
            })
        } else {
            Err(ErrorKind::Unsupported.cause(format!("Unknown method: {:?}", method)))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Class {
    Request = 0b00,
    Indication = 0b01,
    SuccessResponse = 0b10,
    ErrorResponse = 0b11,
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
pub struct Indication<M, A>(Message<M, A>);
impl<M, A> Indication<M, A> {
    pub fn into_inner(self) -> Message<M, A> {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request<M, A>(Message<M, A>);
impl<M, A> Request<M, A>
    where M: Method,
          A: Attribute
{
    pub fn new(method: M) -> Self {
        Request(Message::new(Type {
            class: Class::Request,
            method: method,
        }))
    }
}
impl<M, A> Request<M, A> {
    pub fn into_inner(self) -> Message<M, A> {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<M, A>(Message<M, A>);
impl<M, A> Response<M, A> {
    pub fn into_inner(self) -> Message<M, A> {
        self.0
    }
}

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
    pub fn method(&self) -> &M {
        &self.message_type.method
    }
    pub fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) {
        self.attributes.push(attribute.into());
    }
    pub fn try_into_raw(self) -> Result<RawMessage> {
        let mut raw = RawMessage::new(Type {
            class: self.class(),
            method: self.method().as_u12(),
        });
        for a in self.attributes.iter() {
            let a = may_fail!(a.encode(&raw))?;
            raw.add_attribute(a);
        }
        Ok(raw)
    }
    pub fn try_from_raw(raw: RawMessage) -> Result<Self> {
        let message_type = may_fail!(Type::from_u16(raw.message_type.as_u16()))?;
        let mut message = Message {
            message_type: message_type,
            transaction_id: raw.transaction_id,
            attributes: Vec::new(),
        };
        for a in raw.attributes.iter() {
            let a = may_fail!(A::decode(a, &raw))?;
            message.add_attribute(a);
        }
        Ok(message)
    }
    pub fn try_into_response(self) -> Result<Response<M, A>> {
        fail_if!(!self.class().is_response(),
                 ErrorKind::Failed,
                 "Not a response message: class={:?}",
                 self.class())?;
        Ok(Response(self))
    }
}
impl RawMessage {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let message_type = may_fail!(reader.read_u16be().map_err(Error::from_cause))?;
        let message_type = may_fail!(Type::from_u16(message_type))?;
        let message_len = may_fail!(reader.read_u16be().map_err(Error::from_cause))?;
        fail_if!(message_len % 4 != 0,
                 ErrorKind::NotStunMessage.cause(
                     format!("Unexpected message length: {} % 4 != 0", message_len)))?;
        let magic_cookie = may_fail!(reader.read_u32be().map_err(Error::from_cause))?;
        fail_if!(magic_cookie != MAGIC_COOKIE,
                 ErrorKind::NotStunMessage.cause(
                     format!("Unexpected magic cookie: actual={}, \
                              expected={}",
                             magic_cookie,
                             MAGIC_COOKIE)))?;

        let mut transaction_id: [u8; 12] = [0; 12];
        may_fail!(reader.read_exact(&mut transaction_id).map_err(Error::from_cause))?;

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
        may_fail!(temp_writer.write_u16be(self.message_type.as_u16()).map_err(Error::from_cause))?;
        may_fail!(temp_writer.write_u16be(0).map_err(Error::from_cause))?; // dummy length
        may_fail!(temp_writer.write_u32be(MAGIC_COOKIE).map_err(Error::from_cause))?;
        may_fail!(temp_writer.write_all(&self.transaction_id).map_err(Error::from_cause))?;
        for attr in self.attributes.iter() {
            attr.write_to(&mut temp_writer)?;
        }
        let attrs_len = temp_writer.get_ref().len() - 20;
        fail_if!(attrs_len >= 0x10000,
                 ErrorKind::Failed,
                 "Too large message length: actual={}, limit=0xFFFF",
                 attrs_len)?;
        temp_writer.set_position(2);
        may_fail!(temp_writer.write_u16be(attrs_len as u16).map_err(Error::from_cause))?;

        let buf = temp_writer.into_inner();
        may_fail!(writer.write_all(&buf).map_err(Error::from_cause))?;
        Ok(())
    }
}
