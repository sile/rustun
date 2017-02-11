use std::io::{Read, Write};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::Failure;

use Result;

pub trait ReadExt: Read {
    fn read_u8(&mut self) -> Result<u8> {
        let v = may_fail!(ReadBytesExt::read_u8(self).map_err(Failure::new))?;
        Ok(v)
    }
    fn read_u16(&mut self) -> Result<u16> {
        let v = may_fail!(ReadBytesExt::read_u16::<BigEndian>(self).map_err(Failure::new))?;
        Ok(v)
    }
    fn read_u32(&mut self) -> Result<u32> {
        let v = may_fail!(ReadBytesExt::read_u32::<BigEndian>(self).map_err(Failure::new))?;
        Ok(v)
    }
}
impl<T: Read> ReadExt for T {}

pub trait WriteExt: Write {
    fn write_u8(&mut self, value: u8) -> Result<()> {
        may_fail!(WriteBytesExt::write_u8(self, value).map_err(Failure::new))?;
        Ok(())
    }
    fn write_u16(&mut self, value: u16) -> Result<()> {
        may_fail!(WriteBytesExt::write_u16::<BigEndian>(self, value).map_err(Failure::new))?;
        Ok(())
    }
    fn write_u32(&mut self, value: u32) -> Result<()> {
        may_fail!(WriteBytesExt::write_u32::<BigEndian>(self, value).map_err(Failure::new))?;
        Ok(())
    }
}
impl<T: Write> WriteExt for T {}
