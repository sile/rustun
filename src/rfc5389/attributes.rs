use std::io::{Read, Write};
use std::net::{self, SocketAddr};

use MAGIC_COOKIE;
use {Result, Attribute, AttributeType};
use io::{ReadExt, WriteExt};
use types::AddressFamily;
use super::constants;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MappedAddress(SocketAddr);
impl MappedAddress {
    pub fn new(addr: SocketAddr) -> Self {
        MappedAddress(addr)
    }
    pub fn address(&self) -> SocketAddr {
        self.0
    }
}
impl Attribute for MappedAddress {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_MAPPED_ADDRESS)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_MAPPED_ADDRESS))?;
        let addr = may_fail!(read_address(reader))?;
        Ok(MappedAddress(addr))
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        write_address(writer, self.address())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct XorMappedAddress(SocketAddr);
impl XorMappedAddress {
    pub fn new(addr: SocketAddr) -> Self {
        XorMappedAddress(addr)
    }
    pub fn address(&self) -> SocketAddr {
        self.0
    }
}
impl Attribute for XorMappedAddress {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_XOR_MAPPED_ADDRESS)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_XOR_MAPPED_ADDRESS))?;
        let addr = may_fail!(read_address(reader))?;
        let addr = match addr.ip() {
            net::IpAddr::V4(ip) => SocketAddr::new(net::IpAddr::V4(ipv4_xor(ip)), addr.port()),
            net::IpAddr::V6(_ip) => unimplemented!(),
        };
        Ok(Self::new(addr))
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let addr = match self.0.ip() {
            net::IpAddr::V4(ip) => SocketAddr::new(net::IpAddr::V4(ipv4_xor(ip)), self.0.port()),
            net::IpAddr::V6(_ip) => unimplemented!(),
        };
        write_address(writer, addr)
    }
}

/// foo,bar
///
/// # TODO
///
/// Support SASLprep(RFC4013)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Username(String);
impl Username {
    pub fn new<T: Into<String>>(name: T) -> Result<Self> {
        let name = name.into();
        fail_if!(name.len() >= 513,
                 "Too long USERNAME value: actual={}, limit={}",
                 name.len(),
                 512)?;
        Ok(Username(name))
    }
    pub fn name(&self) -> &str {
        &self.0
    }
}
impl Attribute for Username {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_USERNAME)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_USERNAME))?;
        let name = may_fail!(reader.read_all_string())?;
        Username::new(name)
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_all_ext(self.0.as_bytes()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageIntegrity([u8; 20]);
impl MessageIntegrity {
    pub fn new(hmac_sha1: [u8; 20]) -> Self {
        MessageIntegrity(hmac_sha1)
    }
    pub fn hmac_sha1(&self) -> [u8; 20] {
        self.0
    }
}
impl Attribute for MessageIntegrity {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_MESSAGE_INTEGRITY)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_MESSAGE_INTEGRITY))?;
        let mut buf = [0; 20];
        may_fail!(reader.read_exact_ext(&mut buf[..]))?;
        Ok(Self::new(buf))
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_all_ext(&self.0[..]))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint(u32);
impl Fingerprint {
    pub fn new(crc32: u32) -> Self {
        Fingerprint(crc32)
    }
    pub fn crc32(&self) -> u32 {
        self.0
    }
}
impl Attribute for Fingerprint {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_FINGERPRINT)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_FINGERPRINT))?;
        let crc32 = may_fail!(reader.read_u32())?;
        Ok(Self::new(crc32))
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_u32(self.0))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    code: u16,
    reason: String,
}
impl ErrorCode {
    pub fn new<T: Into<String>>(code: u16, reason: T) -> Result<Self> {
        let reason = reason.into();
        fail_if!(code < 300 || 600 <= code,
                 "Error code {} is out of range",
                 code)?;
        fail_if!(reason.chars().count() >= 128,
                 "Too long reason phrase: actual={} chars, limit=128 chars",
                 reason.chars().count())?;
        Ok(ErrorCode {
            code: code,
            reason: reason,
        })
    }
    pub fn code(&self) -> u16 {
        self.code
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
}
impl Attribute for ErrorCode {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_ERROR_CODE)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_ERROR_CODE))?;
        let v = may_fail!(reader.read_u32())?;
        let class = (v >> 20) & 0b111;
        let number = v >> 23;
        fail_if!(number >= 100,
                 "Too large number part: actual={}, limit=99",
                 number)?;
        let code = ((class * 100) + number) as u16;
        let reason = may_fail!(reader.read_all_string())?;
        Self::new(code, reason)
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let class = (self.code / 100) as u32;
        let number = (self.code % 100) as u32;
        let value = (class << 20) | (number << 23);
        may_fail!(writer.write_u32(value))?;
        may_fail!(writer.write_all_ext(self.reason.as_bytes()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Realm(String);
impl Realm {
    // TODO: validate (RFC3261, RFC4013)
    pub fn new<T: Into<String>>(value: T) -> Result<Self> {
        let value = value.into();
        fail_if!(value.chars().count() >= 128,
                 "Too long realm value: actual={} chars, limit=128 chars",
                 value.chars().count())?;
        Ok(Realm(value))
    }
    pub fn value(&self) -> &str {
        &self.0
    }
}
impl Attribute for Realm {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_REALM)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_REALM))?;
        let value = may_fail!(reader.read_all_string())?;
        Self::new(value)
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_all_ext(self.0.as_bytes()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nonce(String);
impl Nonce {
    // TODO: validate (RFC3261)
    pub fn new<T: Into<String>>(value: T) -> Result<Self> {
        let value = value.into();
        fail_if!(value.chars().count() >= 128,
                 "Too long nonce value: actual={} chars, limit=128 chars",
                 value.chars().count())?;
        Ok(Nonce(value))
    }
    pub fn value(&self) -> &str {
        &self.0
    }
}
impl Attribute for Nonce {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_NONCE)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_NONCE))?;
        let value = may_fail!(reader.read_all_string())?;
        Self::new(value)
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_all_ext(self.0.as_bytes()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnknownAttributes(Vec<AttributeType>);
impl UnknownAttributes {
    pub fn new(attrs: Vec<AttributeType>) -> Self {
        UnknownAttributes(attrs)
    }
    pub fn attributes(&self) -> &[AttributeType] {
        &self.0
    }
}
impl Attribute for UnknownAttributes {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_UNKNOWN_ATTRIBUTES)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_UNKNOWN_ATTRIBUTES))?;
        let mut attrs = Vec::new();
        while let Some(value) = may_fail!(reader.read_u16_or_eof())? {
            attrs.push(AttributeType::new(value));
        }
        Ok(Self::new(attrs))
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        for a in self.0.iter() {
            may_fail!(writer.write_u16(a.as_u16()))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Software(String);
impl Software {
    pub fn new<T: Into<String>>(value: T) -> Result<Self> {
        let value = value.into();
        fail_if!(value.chars().count() >= 128,
                 "Too long software value: actual={} chars, limit=128 chars",
                 value.chars().count())?;
        Ok(Software(value))
    }
    pub fn value(&self) -> &str {
        &self.0
    }
}
impl Attribute for Software {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_SOFTWARE)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_SOFTWARE))?;
        let value = may_fail!(reader.read_all_string())?;
        Self::new(value)
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        may_fail!(writer.write_all_ext(self.0.as_bytes()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlternateServer(SocketAddr);
impl AlternateServer {
    pub fn new(addr: SocketAddr) -> Self {
        AlternateServer(addr)
    }
    pub fn address(&self) -> SocketAddr {
        self.0
    }
}
impl Attribute for AlternateServer {
    fn get_type(&self) -> AttributeType {
        AttributeType::new(constants::ATTR_TYPE_ALTERNATE_SERVER)
    }
    fn read_value_from<R: Read>(attr_type: AttributeType, reader: &mut R) -> Result<Self> {
        may_fail!(attr_type.expect(constants::ATTR_TYPE_MAPPED_ADDRESS))?;
        let addr = may_fail!(read_address(reader))?;
        Ok(Self::new(addr))
    }
    fn write_value_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        write_address(writer, self.address())
    }
}

fn read_address<R: Read>(reader: &mut R) -> Result<SocketAddr> {
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
            may_fail!(reader.read_exact_ext(&mut buf[..]))?;
            let ip = net::Ipv6Addr::from(buf);
            net::IpAddr::V6(net::Ipv6Addr::from(ip))
        }
    };
    Ok(SocketAddr::new(ip, port))
}

fn write_address<W: Write>(writer: &mut W, addr: SocketAddr) -> Result<()> {
    may_fail!(writer.write_u8(0))?;
    may_fail!(AddressFamily::from(addr).write_to(writer))?;
    may_fail!(writer.write_u16(addr.port()))?;
    match addr.ip() {
        net::IpAddr::V4(ip) => {
            may_fail!(writer.write_all_ext(&ip.octets()))?;
        }
        net::IpAddr::V6(ip) => {
            may_fail!(writer.write_all_ext(&ip.octets()))?;
        }
    }
    Ok(())
}

fn ipv4_xor(ip: net::Ipv4Addr) -> net::Ipv4Addr {
    let mut xor = [0; 4];
    for (i, o) in ip.octets().iter().enumerate() {
        xor[i] = *o ^ (MAGIC_COOKIE >> (24 - i * 8)) as u8;
    }
    From::from(xor)
}
