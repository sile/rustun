use std::io::{Read, Write};
use std::net::{self, SocketAddr};
use failure::Failure;

use io::{ReadExt, WriteExt};
use {Result, Attribute, AttributeType};
use super::{TYPE_MAPPED_ADDRESS, AddressFamily};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MappedAddress(SocketAddr);
impl MappedAddress {
    pub fn address(&self) -> SocketAddr {
        self.0
    }
}
impl Attribute for MappedAddress {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(TYPE_MAPPED_ADDRESS)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(TYPE_MAPPED_ADDRESS))?;
        let _ = may_fail!(reader.read_u8())?;
        let family = may_fail!(AddressFamily::read_from(reader))?;
        let port = may_fail!(reader.read_u16())?;
        let ip = match family {
            AddressFamily::Ipv4 => {
                let ip = may_fail!(reader.read_u32())?;
                let ip = net::Ipv4Addr::from(ip);
                net::IpAddr::V4(net::Ipv4Addr::from(ip))
            }
            AddressFamily::Ipv6 => {
                let mut buf = [0; 16];
                may_fail!(reader.read_exact(&mut buf[..]).map_err(Failure::new))?;
                let ip = net::Ipv6Addr::from(buf);
                net::IpAddr::V6(net::Ipv6Addr::from(ip))
            }
        };
        Ok(MappedAddress(SocketAddr::new(ip, port)))
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let addr = self.address();
        may_fail!(writer.write_u8(0))?;
        may_fail!(AddressFamily::from(addr).write_to(writer))?;
        may_fail!(writer.write_u16(addr.port()))?;
        match addr.ip() {
            net::IpAddr::V4(ip) => {
                may_fail!(writer.write_all(&ip.octets()).map_err(Failure::new))?;
            }
            net::IpAddr::V6(ip) => {
                may_fail!(writer.write_all(&ip.octets()).map_err(Failure::new))?;
            }
        }
        Ok(())
    }
}
