use std::io::{Read, Write, Cursor};
use rand;

pub use message::{Class, Type};

use {Result, Error, StunMethod, MAGIC_COOKIE};
use types::U12;
use attr::{Attribute, RawAttribute};
use message::TransactionId;
use io::{WriteExt, ReadExt};

pub type RawMessage = Message<U12, RawAttribute>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<M, A> {
    message_type: Type<M>,
    transaction_id: [u8; 12],
    attributes: Vec<A>,
}
impl<M, A> Message<M, A>
    where M: StunMethod,
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
        may_fail!(reader.read_exact_ext(&mut transaction_id))?;

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
        may_fail!(temp_writer.write_all_ext(&self.transaction_id))?;
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
        may_fail!(writer.write_all_ext(&buf))?;
        Ok(())
    }
}
