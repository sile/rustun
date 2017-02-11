use std::io::{Read, Write};
use failure::Failure;

use Result;
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

pub trait Attribute: Sized {
    fn get_type(&self) -> AttributeType;
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let attr_type = may_fail!(reader.read_u16())?;
        let attr_type = AttributeType::new(attr_type);
        let value_len = may_fail!(reader.read_u16(),
                                  "Cannot read STUN attribute(type={}) length",
                                  attr_type.as_u16())?;
        let this = {
            let mut value_reader = reader.take(value_len as u64);
            let this = may_fail!(Self::read_value_from(attr_type, &mut value_reader),
                                 "Cannot read STUN attribute(type={}) value",
                                 attr_type.as_u16())?;
            fail_if!(value_reader.limit() != 0,
                     "Value bytes for STUN attribute(type={}) was not fully consumed: \
                               bytes={}, remaings={}",
                     attr_type.as_u16(),
                     value_len,
                     value_reader.limit())?;
            this
        };
        let padding_len = ((4 - value_len % 4) % 4) as usize;
        may_fail!(reader.read_exact(&mut [0; 4][0..padding_len]).map_err(Failure::new))?;
        Ok(this)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self>;
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_u16(self.get_type().as_u16()),
                  "Cannot write STUN attribute type: type={}",
                  self.get_type().as_u16())?;

        let mut value_buf = Vec::new();
        may_fail!(self.write_value_to(&mut value_buf),
                  "Cannot write STUN attribute(type={}) value bytes",
                  self.get_type().as_u16())?;
        fail_if!(value_buf.len() > 0xFFFF,
                 "Too large STUN attribute(type={}) value bytes: length={}, limit=0xFFFF",
                 self.get_type().as_u16(),
                 value_buf.len())?;

        may_fail!(writer.write_u16(value_buf.len() as u16),
                  "Cannot write STUN attribute(type={}) length",
                  self.get_type().as_u16())?;

        let padded_len = value_buf.len() + (4 - value_buf.len() % 4) % 4;
        value_buf.resize(padded_len, 0);
        may_fail!(writer.write_all(&value_buf).map_err(Failure::new),
                  "Cannot write STUN attribute(type={}) value bytes: length={}",
                  self.get_type().as_u16(),
                  value_buf.len())?;
        Ok(())
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()>;
}
