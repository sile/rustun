use std::io::{Read, Write};
use std::net::{self, SocketAddr};

use io::{ReadExt, WriteExt};
use Result;

pub use self::mapped_address::MappedAddress;

mod mapped_address;

pub const TYPE_MAPPED_ADDRESS: u16 = 0x0001;
pub const TYPE_USERNAME: u16 = 0x0006;
pub const TYPE_MESSAGE_INTEGRITY: u16 = 0x0008;
pub const TYPE_ERROR_CODE: u16 = 0x0009;
pub const TYPE_UNKNOWN_ATTRIBUTES: u16 = 0x000A;
pub const TYPE_REALM: u16 = 0x0014;
pub const TYPE_NONCE: u16 = 0x0015;
pub const TYPE_XOR_MAPPED_ADDRESS: u16 = 0x0020;
pub const TYPE_SOFTWARE: u16 = 0x8022;
pub const TYPE_ALTERNATE_SERVER: u16 = 0x8023;
pub const TYPE_FINGERPRINT: u16 = 0x8028;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
enum AddressFamily {
    Ipv4 = 1,
    Ipv6 = 2,
}
impl AddressFamily {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let family = may_fail!(reader.read_u8())?;
        match family {
            1 => Ok(AddressFamily::Ipv4),
            2 => Ok(AddressFamily::Ipv6),
            _ => failed!("Unknown address family: {}", family)?,
        }
    }
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_u8(*self as u8))?;
        Ok(())
    }
}
impl From<SocketAddr> for AddressFamily {
    fn from(f: SocketAddr) -> Self {
        match f.ip() {
            net::IpAddr::V4(_) => AddressFamily::Ipv4,
            net::IpAddr::V6(_) => AddressFamily::Ipv6,
        }
    }
}
