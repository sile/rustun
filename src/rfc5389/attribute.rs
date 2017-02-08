use std::io::{Read, Write};
use std::net::SocketAddr;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;

use MAGIC_COOKIE;
use attribute;
use {Result, ErrorKind};

pub const TYPE_XOR_MAPPED_ADDRESS: u16 = 0x0020;

#[derive(Debug)]
pub enum Attribute {
    XorMappedAddress(XorMappedAddress),
}
impl attribute::Attribute for Attribute {
    fn get_type(&self) -> u16 {
        match *self {
            Attribute::XorMappedAddress(_) => TYPE_XOR_MAPPED_ADDRESS,
        }
    }
    fn read_value_from<R: Read>(attr_type: u16, reader: &mut R) -> Result<Self> {
        match attr_type {
            TYPE_XOR_MAPPED_ADDRESS => {
                XorMappedAddress::read_from(reader).map(Attribute::XorMappedAddress)
            }
            _ => Err(ErrorKind::UnknownAttribute(attr_type).into()),
        }
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        match *self {
            Attribute::XorMappedAddress(ref a) => a.write_to(writer),
        }
    }
}

#[derive(Debug)]
pub struct XorMappedAddress {
    pub address: SocketAddr,
}
impl XorMappedAddress {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let _ = reader.read_u8()?;
        let family = reader.read_u8()?;
        let port = reader.read_u16::<BigEndian>()?;
        assert!(family == 1 || family == 2);
        if family == 1 {
            let x_addr = reader.read_u32::<BigEndian>()?;
            let addr = ::std::net::Ipv4Addr::from(x_addr ^ MAGIC_COOKIE);
            Ok(XorMappedAddress {
                address: SocketAddr::V4(::std::net::SocketAddrV4::new(addr, port)),
            })
        } else {
            // TODO: xor
            let mut buf = [0; 16];
            reader.read_exact(&mut buf[..])?;
            let addr = ::std::net::Ipv6Addr::from(buf);
            Ok(XorMappedAddress {
                address: SocketAddr::V6(::std::net::SocketAddrV6::new(addr, port, 0, 0)),
            })
        }
    }
    fn write_to<W: Write>(&self, _writer: &mut W) -> Result<()> {
        panic!()
    }
}



// #[derive(Debug, Clone, Copy)]
// pub enum AttrType {
//     MappedAddress,
//     Username,
//     MessageIntegrity,
//     ErrorCode,
//     UnknownAttributes,
//     Realm,
//     Nonce,
//     XorMappedAddress,
//     Software,
//     AlternateServer,
//     Fingerprint,
//     Other(u16),
// }
// impl AttrType {
//     pub fn from_u16(value: u16) -> Self {
//         match value {
//             0x0001 => AttrType::MappedAddress,
//             0x0006 => AttrType::Username,
//             0x0008 => AttrType::MessageIntegrity,
//             0x0009 => AttrType::ErrorCode,
//             0x000A => AttrType::UnknownAttributes,
//             0x0014 => AttrType::Realm,
//             0x0015 => AttrType::Nonce,
//             0x0020 => AttrType::XorMappedAddress,
//             0x8022 => AttrType::Software,
//             0x8023 => AttrType::AlternateServer,
//             0x8028 => AttrType::Fingerprint,
//             other => AttrType::Other(other),
//         }
//     }
//     pub fn as_u16(&self) -> u16 {
//         match *self {
//             AttrType::MappedAddress => 0x0001,
//             AttrType::Username => 0x0006,
//             AttrType::MessageIntegrity => 0x0008,
//             AttrType::ErrorCode => 0x0009,
//             AttrType::UnknownAttributes => 0x000A,
//             AttrType::Realm => 0x0014,
//             AttrType::Nonce => 0x0015,
//             AttrType::XorMappedAddress => 0x0020,
//             AttrType::Software => 0x0022,
//             AttrType::AlternateServer => 0x0023,
//             AttrType::Fingerprint => 0x8028,
//             AttrType::Other(value) => value,
//         }
//     }
//     pub fn is_comprehension_required(&self) -> bool {
//         self.as_u16() < 0x8000
//     }
// }
