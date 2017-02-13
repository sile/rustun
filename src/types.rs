use std::io::{Read, Write};
use std::net::{SocketAddr, IpAddr};

use Result;
use io::{ReadExt, WriteExt};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct U12(u16);
impl U12 {
    pub fn from_u8(value: u8) -> Self {
        U12(value as u16)
    }
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

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum AddressFamily {
    Ipv4 = 1,
    Ipv6 = 2,
}
impl AddressFamily {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let family = may_fail!(reader.read_u8())?;
        match family {
            1 => Ok(AddressFamily::Ipv4),
            2 => Ok(AddressFamily::Ipv6),
            _ => failed!("Unknown address family: {}", family)?,
        }
    }
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_u8(*self as u8))?;
        Ok(())
    }
}
impl From<SocketAddr> for AddressFamily {
    fn from(f: SocketAddr) -> Self {
        match f.ip() {
            IpAddr::V4(_) => AddressFamily::Ipv4,
            IpAddr::V6(_) => AddressFamily::Ipv6,
        }
    }
}
