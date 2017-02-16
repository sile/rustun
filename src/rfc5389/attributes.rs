use std::io::{Read, Write};
use std::net::{SocketAddr, IpAddr};
use handy_async::sync_io::{ReadExt, WriteExt};
use track_err::ErrorKindExt;

use {Result, Error, Attribute, ErrorKind};
use message::RawMessage;
use attribute::{Type, RawAttribute};
use constants;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XorMappedAddress(SocketAddr);
impl XorMappedAddress {
    pub fn new(addr: SocketAddr) -> Self {
        XorMappedAddress(addr)
    }
    pub fn address(&self) -> SocketAddr {
        self.0
    }
    fn xor_addr(addr: SocketAddr) -> SocketAddr {
        match addr.ip() {
            IpAddr::V4(ip) => {
                let mut octets = ip.octets();
                for i in 0..octets.len() {
                    octets[i] ^= (constants::MAGIC_COOKIE >> (24 - i * 8)) as u8;
                }
                let xor_ip = From::from(octets);
                SocketAddr::new(IpAddr::V4(xor_ip), addr.port())
            }
            IpAddr::V6(_ip) => panic!(),
        }
    }
}
impl Attribute for XorMappedAddress {
    fn get_type(&self) -> Type {
        Type::new(TYPE_XOR_MAPPED_ADDRESS)
    }
    fn decode(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        fail_if_ne!(attr.get_type().as_u16(),
                    TYPE_XOR_MAPPED_ADDRESS,
                    ErrorKind::Failed)?;
        let xor_addr = may_fail!(read_socket_addr(&mut attr.value()))?;
        let addr = Self::xor_addr(xor_addr);
        Ok(Self::new(addr))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let xor_addr = Self::xor_addr(self.0);
        let mut buf = Vec::new();
        may_fail!(write_socket_addr(&mut buf, xor_addr))?;
        Ok(buf)
    }
}

fn read_socket_addr<R: Read>(reader: &mut R) -> Result<SocketAddr> {
    let _ = may_fail!(reader.read_u8().map_err(Error::from_cause))?;
    let family = may_fail!(reader.read_u8().map_err(Error::from_cause))?;
    let port = may_fail!(reader.read_u16be().map_err(Error::from_cause))?;
    let ip = match family {
        1 => {
            let ip = may_fail!(reader.read_u32be().map_err(Error::from_cause))?;
            IpAddr::V4(From::from(ip))
        }
        2 => {
            let mut octets = [0; 16];
            may_fail!(reader.read_exact(&mut octets[..]).map_err(Error::from_cause))?;
            IpAddr::V6(From::from(octets))
        }
        _ => {
            let message = format!("Unsupported address family: {}", family);
            return Err(ErrorKind::Unsupported.cause(message));
        }
    };
    Ok(SocketAddr::new(ip, port))
}

fn write_socket_addr<W: Write>(writer: &mut W, addr: SocketAddr) -> Result<()> {
    may_fail!(writer.write_u8(0).map_err(Error::from_cause))?;
    match addr.ip() {
        IpAddr::V4(ip) => {
            may_fail!(writer.write_u8(1).map_err(Error::from_cause))?;
            may_fail!(writer.write_u16be(addr.port()).map_err(Error::from_cause))?;
            may_fail!(writer.write_all(&ip.octets()).map_err(Error::from_cause))?;
        }
        IpAddr::V6(ip) => {
            may_fail!(writer.write_u8(2).map_err(Error::from_cause))?;
            may_fail!(writer.write_u16be(addr.port()).map_err(Error::from_cause))?;
            may_fail!(writer.write_all(&ip.octets()).map_err(Error::from_cause))?;
        }
    }
    Ok(())
}
