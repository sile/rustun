use std::io::{self, Read, Write};
use handy_async::sync_io::{ReadExt, WriteExt};

use {Result, Method, Attribute, ErrorKind};
use types::{U12, TransactionId};
use attribute::RawAttribute;
use message::{Request, Indication, SuccessResponse, ErrorResponse, Response};
use constants::MAGIC_COOKIE;

#[derive(Debug, Clone)]
pub struct RawMessage {
    class: Class,
    method: U12,
    transaction_id: TransactionId,
    attributes: Vec<RawAttribute>,
}
impl RawMessage {
    pub fn new(class: Class,
               method: U12,
               transaction_id: TransactionId,
               attributes: Vec<RawAttribute>)
               -> Self {
        RawMessage {
            class: class,
            method: method,
            transaction_id: transaction_id,
            attributes: attributes,
        }
    }
    pub fn class(&self) -> Class {
        self.class
    }
    pub fn method(&self) -> U12 {
        self.method
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("must succeed");
        buf
    }
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut temp_writer = io::Cursor::new(vec![0; 20]);
        let message_type = Type {
            class: self.class,
            method: self.method,
        };
        track_try!(temp_writer.write_u16be(message_type.as_u16()));
        track_try!(temp_writer.write_u16be(0)); // dummy length
        track_try!(temp_writer.write_u32be(MAGIC_COOKIE));
        track_try!(temp_writer.write_all(&self.transaction_id));
        for attr in self.attributes.iter() {
            track_try!(attr.write_to(&mut temp_writer));
        }
        let attrs_len = temp_writer.get_ref().len() - 20;
        track_assert!(attrs_len < 0x10000,
                      ErrorKind::Failed,
                      "Too large message length: actual={}, limit=0xFFFF",
                      attrs_len);
        temp_writer.set_position(2);
        track_try!(temp_writer.write_u16be(attrs_len as u16));

        let buf = temp_writer.into_inner();
        track_try!(writer.write_all(&buf));
        Ok(())
    }
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let message_type = track_try!(reader.read_u16be());
        let message_type = track_try!(Type::from_u16(message_type));
        let message_len = track_try!(reader.read_u16be());
        track_assert!(message_len % 4 == 0,
                      ErrorKind::NotStunMessage,
                      "Unexpected message length: {} % 4 != 0",
                      message_len);
        let magic_cookie = track_try!(reader.read_u32be());
        track_assert!(magic_cookie == MAGIC_COOKIE,
                      ErrorKind::NotStunMessage,
                      "Unexpected magic cookie: actual={}, expected={}",
                      magic_cookie,
                      MAGIC_COOKIE);;

        let mut transaction_id: [u8; 12] = [0; 12];
        track_try!(reader.read_exact(&mut transaction_id));

        let mut attrs = Vec::new();
        let mut reader = reader.take(message_len as u64);
        while reader.limit() > 0 {
            let attr = track_try!(RawAttribute::read_from(&mut reader));
            attrs.push(attr);
        }
        Ok(RawMessage::new(message_type.class,
                           message_type.method,
                           transaction_id,
                           attrs))
    }

    pub fn try_into_request<M, A>(self) -> Result<Request<M, A>>
        where M: Method,
              A: Attribute
    {
        track_assert_eq!(self.class, Class::Request, ErrorKind::Other);
        let method = track_try!(M::from_u12(self.method).ok_or(ErrorKind::Other));
        let mut attrs = Vec::new();
        for a in self.attributes.iter() {
            let a = track_try!(A::decode(&a, &self));
            attrs.push(a);
        }
        Ok(Request {
            method: method,
            transaction_id: self.transaction_id,
            attributes: attrs,
        })
    }
    pub fn try_into_indication<M, A>(self) -> Result<Indication<M, A>>
        where M: Method,
              A: Attribute
    {
        track_assert_eq!(self.class, Class::Indication, ErrorKind::Other);
        let method = track_try!(M::from_u12(self.method).ok_or(ErrorKind::Other));
        let mut attrs = Vec::new();
        for a in self.attributes.iter() {
            let a = track_try!(A::decode(&a, &self));
            attrs.push(a);
        }
        Ok(Indication {
            method: method,
            transaction_id: self.transaction_id,
            attributes: attrs,
        })
    }
    pub fn try_into_response<M, A>(self) -> Result<Response<M, A>>
        where M: Method,
              A: Attribute
    {
        track_assert!(self.class == Class::SuccessResponse || self.class == Class::ErrorResponse,
                      ErrorKind::Other);
        let method = track_try!(M::from_u12(self.method).ok_or(ErrorKind::Other));
        let mut attrs = Vec::new();
        for a in self.attributes.iter() {
            let a = track_try!(A::decode(&a, &self));
            attrs.push(a);
        }
        if self.class == Class::SuccessResponse {
            Ok(Ok(SuccessResponse {
                method: method,
                transaction_id: self.transaction_id,
                attributes: attrs,
            }))
        } else {
            panic!()
            // TODO: handle error code attribute
            // Ok(Err(ErrorResponse {
            //     method: method,
            //     transaction_id: self.transaction_id,
            //     attributes: Vec::new(),
            // }))
        }
    }
    pub fn try_from_request<M, A>(from: Request<M, A>) -> Result<Self>
        where M: Method,
              A: Attribute
    {
        track_err!(Self::try_from(Class::Request,
                                  from.method(),
                                  *from.transaction_id(),
                                  from.attributes()))
    }
    pub fn try_from_indication<M, A>(from: Indication<M, A>) -> Result<Self>
        where M: Method,
              A: Attribute
    {
        track_err!(Self::try_from(Class::Indication,
                                  from.method(),
                                  *from.transaction_id(),
                                  from.attributes()))
    }
    pub fn try_from_success_response<M, A>(from: SuccessResponse<M, A>) -> Result<Self>
        where M: Method,
              A: Attribute
    {
        track_err!(Self::try_from(Class::SuccessResponse,
                                  from.method(),
                                  *from.transaction_id(),
                                  from.attributes()))
    }
    pub fn try_from_error_response<M, A>(from: ErrorResponse<M, A>) -> Result<Self>
        where M: Method,
              A: Attribute
    {
        track_err!(Self::try_from(Class::ErrorResponse,
                                  from.method(),
                                  *from.transaction_id(),
                                  from.attributes()))
    }

    fn try_from<M, A>(class: Class,
                      method: &M,
                      transaction_id: TransactionId,
                      attributes: &[A])
                      -> Result<Self>
        where M: Method,
              A: Attribute
    {
        let mut m = RawMessage::new(class, method.as_u12(), transaction_id, Vec::new());
        for attr in attributes {
            let raw_attr = track_try!(attr.encode(&m));
            m.attributes.push(raw_attr);
        }
        Ok(m)
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Type {
    pub class: Class,
    pub method: U12,
}
impl Type {
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
        track_assert!(value >> 14 == 0,
                      ErrorKind::NotStunMessage,
                      "First two-bits of STUN message must be 0");
        let class = ((value >> 4) & 0b01) | ((value >> 7) & 0b10);
        let class = Class::from_u8(class as u8).unwrap();
        let method = (value & 0b0000_0000_1111) | ((value >> 1) & 0b0000_0111_0000) |
                     ((value >> 2) & 0b1111_1000_0000);
        let method = U12::from_u16(method).unwrap();
        Ok(Type {
            class: class,
            method: method,
        })
    }
}
