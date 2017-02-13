use std::io::{Read, Write};

use Result;
use msg::RawMessage;
use io::{ReadExt, WriteExt};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct AttributeType(u16);
impl AttributeType {
    pub fn new(type_u16: u16) -> Self {
        AttributeType(type_u16)
    }
    pub fn as_u16(&self) -> u16 {
        self.0
    }
    pub fn is_comprehension_required(&self) -> bool {
        self.0 < 0x8000
    }
    pub fn is_comprehension_optional(&self) -> bool {
        !self.is_comprehension_required()
    }
    pub fn expect<T: Into<Self>>(&self, expected: T) -> Result<()> {
        let expected = expected.into();
        fail_if!(*self != expected,
                 "Unexpected attribute type: actual={}, expected={}",
                 self.0,
                 expected.0)?;
        Ok(())
    }
}
impl From<u16> for AttributeType {
    fn from(f: u16) -> Self {
        Self::new(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RawAttribute {
    attr_type: AttributeType,
    value: Vec<u8>,
    padding: [u8; 4],
}
impl RawAttribute {
    pub fn new(attr_type: AttributeType, value: Vec<u8>) -> Self {
        assert!(value.len() < 0x10000);
        RawAttribute {
            attr_type: attr_type,
            value: value,
            padding: [0; 4],
        }
    }
    pub fn attr_type(&self) -> AttributeType {
        self.attr_type
    }
    pub fn value(&self) -> &[u8] {
        &self.value
    }
    pub fn padding(&self) -> &[u8] {
        let padding_len = (4 - self.value.len() % 4) % 4;
        &self.padding[..padding_len]
    }
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let attr_type = may_fail!(reader.read_u16())?;
        let attr_type = AttributeType::new(attr_type);
        let value_len = may_fail!(reader.read_u16())? as u64;
        let value = may_fail!(reader.take(value_len).read_all_bytes())?;
        let mut padding = [0; 4];
        let padding_len = ((4 - value_len % 4) % 4) as usize;
        may_fail!(reader.read_exact_ext(&mut padding[..padding_len]))?;
        Ok(RawAttribute {
            attr_type: attr_type,
            value: value,
            padding: padding,
        })
    }
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_u16(self.attr_type.as_u16()))?;
        may_fail!(writer.write_u16(self.value.len() as u16))?;
        may_fail!(writer.write_all_ext(&self.value))?;
        may_fail!(writer.write_all_ext(self.padding()))?;
        Ok(())
    }
}

pub trait Attribute: Sized {
    fn get_type(&self) -> AttributeType;
    fn decode(attr: &RawAttribute, message: &RawMessage) -> Result<Self>;
    fn encode(&self, message: &RawMessage) -> Result<RawAttribute>;
}
