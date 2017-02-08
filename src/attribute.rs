use std::io::{Read, Write};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use {Result, ResultExt};

pub trait Attribute: Sized {
    fn get_type(&self) -> u16;
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let attr_type = reader.read_u16::<BigEndian>()
            .chain_err(|| "Cannot read STUN attribute type")?;
        let value_len = reader.read_u16::<BigEndian>()
            .chain_err(|| format!("Cannot read STUN attribute(type={}) length", attr_type))?;
        let this = {
            let mut value_reader = reader.take(value_len as u64);
            let this = Self::read_value_from(attr_type, &mut value_reader)
                .chain_err(|| format!("Cannot read STUN attribute(type={}) value", attr_type))?;
            if value_reader.limit() != 0 {
                bail!("Value bytes for STUN attribute(type={}) was not fully consumed: \
                               bytes={}, remaings={}",
                      attr_type,
                      value_len,
                      value_reader.limit());
            }
            this
        };
        let padding_len = ((4 - value_len % 4) % 4) as usize;
        reader.read_exact(&mut [0; 4][0..padding_len])?;
        Ok(this)
    }
    fn read_value_from<R: Read>(attr_type: u16, reader: &mut R) -> Result<Self>;
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u16::<BigEndian>(self.get_type())
            .chain_err(|| format!("Cannot write STUN attribute type: type={}", self.get_type()))?;

        let mut value_buf = Vec::new();
        self.write_value_to(&mut value_buf)
            .chain_err(|| {
                format!("Cannot write STUN attribute(type={}) value bytes",
                        self.get_type())
            })?;
        if value_buf.len() > 0xFFFF {
            bail!("Too large STUN attribute(type={}) value bytes: length={}, limit=0xFFFF",
                  self.get_type(),
                  value_buf.len());
        }

        writer.write_u16::<BigEndian>(value_buf.len() as u16)
            .chain_err(|| {
                format!("Cannot write STUN attribute(type={}) length",
                        self.get_type())
            })?;

        let padded_len = value_buf.len() + (4 - value_buf.len() % 4) % 4;
        value_buf.resize(padded_len, 0);
        writer.write_all(&value_buf)
            .chain_err(|| {
                format!("Cannot write STUN attribute(type={}) value bytes: length={}",
                        self.get_type(),
                        value_buf.len())
            })?;
        Ok(())
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()>;
}
