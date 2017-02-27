//! Individual Definition of the attributes that are defined in [RFC 5389]
//! (https://tools.ietf.org/html/rfc5389).
use std::io::{Read, Write, Cursor};
use std::net::{SocketAddr, IpAddr};
use handy_async::sync_io::{ReadExt, WriteExt};
use trackable::error::ErrorKindExt;

use {Result, Attribute, ErrorKind};
use message::RawMessage;
use attribute::{Type, RawAttribute};
use constants;

/// The codepoint of the [MappedAddress](struct.MappedAddress.html) attribute.
pub const TYPE_MAPPED_ADDRESS: u16 = 0x0001;

/// The codepoint of the [Username](struct.Username.html) attribute.
pub const TYPE_USERNAME: u16 = 0x0006;

/// The codepoint of the [MessageIntegrity](struct.MessageIntegrity.html) attribute.
pub const TYPE_MESSAGE_INTEGRITY: u16 = 0x0008;

/// The codepoint of the [ErrorCode](struct.ErrorCode.html) attribute.
pub const TYPE_ERROR_CODE: u16 = 0x0009;

/// The codepoint of the [UnknownAttributes](struct.UnknownAttributes.html) attribute.
pub const TYPE_UNKNOWN_ATTRIBUTES: u16 = 0x000A;

/// The codepoint of the [Realm](struct.Realm.html) attribute.
pub const TYPE_REALM: u16 = 0x0014;

/// The codepoint of the [Nonce](struct.Nonce.html) attribute.
pub const TYPE_NONCE: u16 = 0x0015;

/// The codepoint of the [XorMappedAddress](struct.XorMappedAddress.html) attribute.
pub const TYPE_XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// The codepoint of the [Software](struct.Software.html) attribute.
pub const TYPE_SOFTWARE: u16 = 0x8022;

/// The codepoint of the [AlternateServer](struct.AlternateServer.html) attribute.
pub const TYPE_ALTERNATE_SERVER: u16 = 0x8023;

/// The codepoint of the [Fingerprint](struct.Fingerprint.html) attribute.
pub const TYPE_FINGERPRINT: u16 = 0x8028;

/// `MAPPED-ADDRESS` attribute.
///
/// See [RFC 5389 -- 15.1. MAPPED-ADDRESS]
/// (https://tools.ietf.org/html/rfc5389#section-15.1) about this attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MappedAddress(SocketAddr);
impl MappedAddress {
    /// Makes a new `MappedAddress` instance.
    pub fn new(addr: SocketAddr) -> Self {
        MappedAddress(addr)
    }

    /// Returns the address of this instance.
    pub fn address(&self) -> SocketAddr {
        self.0
    }
}
impl Attribute for MappedAddress {
    fn get_type(&self) -> Type {
        Type::new(TYPE_MAPPED_ADDRESS)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_MAPPED_ADDRESS,
                         ErrorKind::Failed);
        let addr = track_try!(read_socket_addr(&mut attr.value()));
        Ok(Self::new(addr))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        track_try!(write_socket_addr(&mut buf, self.0));
        Ok(buf)
    }
}

/// `ALTERNATE-SERVER` attribute.
///
/// See [RFC 5389 -- 15.11. ALTERNATE-SERVER]
/// (https://tools.ietf.org/html/rfc5389#section-15.11) about this attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AlternateServer(SocketAddr);
impl AlternateServer {
    /// Makes a new `AlternateServer` instance.
    pub fn new(addr: SocketAddr) -> Self {
        AlternateServer(addr)
    }

    /// Returns the alternate address.
    pub fn address(&self) -> SocketAddr {
        self.0
    }
}
impl Attribute for AlternateServer {
    fn get_type(&self) -> Type {
        Type::new(TYPE_ALTERNATE_SERVER)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_ALTERNATE_SERVER,
                         ErrorKind::Failed);
        let addr = track_try!(read_socket_addr(&mut attr.value()));
        Ok(Self::new(addr))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        track_try!(write_socket_addr(&mut buf, self.0));
        Ok(buf)
    }
}

/// `USERNAME` attribute.
///
/// See [RFC 5389 -- 15.3. USERNAME]
/// (https://tools.ietf.org/html/rfc5389#section-15.3) about this attribute.
///
/// # TODO
///
/// - Support SASLprep [RFC 4013]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Username {
    name: String,
}
impl Username {
    /// Makes a new `Username` instance.
    ///
    /// The length of `name` must be less then `513` bytes.
    /// If it is too long, this will return `None`.
    pub fn new(name: String) -> Option<Self> {
        if name.len() < 513 {
            Some(Username { name: name })
        } else {
            None
        }
    }

    /// Returns the name of this instance.
    pub fn name(&self) -> &str {
        &self.name
    }
}
impl Attribute for Username {
    fn get_type(&self) -> Type {
        Type::new(TYPE_USERNAME)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(), TYPE_USERNAME, ErrorKind::Failed);
        let name = track_try!((&mut attr.value()).read_all_string());
        track_assert!(name.len() < 513, ErrorKind::Other);
        Ok(Self::new(name).unwrap())
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        Ok(Vec::from(self.name.as_bytes()))
    }
}


/// `MESSAGE-INTEGRITY` attribute.
///
/// See [RFC 5389 -- 15.3. MESSAGE-INTEGRITY]
/// (https://tools.ietf.org/html/rfc5389#section-15.4) about this attribute.
///
/// # TODO
///
/// - Check integrity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageIntegrity {
    hmac_sha1: [u8; 20],
}
impl MessageIntegrity {
    /// Makes a new `MessageIntegrity` instance.
    pub fn new(hmac_sha1: [u8; 20]) -> Self {
        MessageIntegrity { hmac_sha1: hmac_sha1 }
    }

    /// Returns the HMAC-SHA1 of this instance.
    pub fn hmac_sha1(&self) -> [u8; 20] {
        self.hmac_sha1
    }
}
impl Attribute for MessageIntegrity {
    fn get_type(&self) -> Type {
        Type::new(TYPE_MESSAGE_INTEGRITY)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_MESSAGE_INTEGRITY,
                         ErrorKind::Failed);
        track_assert_eq!(attr.value().len(), 20, ErrorKind::Other);
        let mut hmac_sha1 = [0; 20];
        (&mut hmac_sha1[..]).copy_from_slice(attr.value());
        Ok(Self::new(hmac_sha1))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        Ok(Vec::from(&self.hmac_sha1[..]))
    }
}

/// `ERROR-CODE` attribute.
///
/// See [RFC 5389 -- 15.6. ERROR-CODE]
/// (https://tools.ietf.org/html/rfc5389#section-15.6) about this attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    code: u16,
    reason_phrase: String,
}
impl ErrorCode {
    /// Makes a new `ErrorCode` instance.
    ///
    /// Note that the value of `code` must be in range of `300..600`.
    /// If the value is out-of-range this will return `None`.
    pub fn new(code: u16, reason_phrase: String) -> Option<Self> {
        if 300 <= code && code < 600 {
            Some(ErrorCode {
                code: code,
                reason_phrase: reason_phrase,
            })
        } else {
            None
        }
    }

    /// Returns the code of this error.
    pub fn code(&self) -> u16 {
        self.code
    }

    /// Returns the reason phrase of this error.
    pub fn reason_phrase(&self) -> &str {
        &self.reason_phrase
    }
}
impl Attribute for ErrorCode {
    fn get_type(&self) -> Type {
        Type::new(TYPE_ERROR_CODE)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(), TYPE_ERROR_CODE, ErrorKind::Failed);
        let mut reader = &mut attr.value();
        let value = track_try!(reader.read_u32be());
        let class = (value >> 8) & 0b111;
        let number = value & 0b11111111;
        track_assert!(3 <= class && class < 6, ErrorKind::Other);
        track_assert!(number < 100, ErrorKind::Other);

        let code = (class * 100 + number) as u16;
        let reason = track_try!(reader.read_all_string());
        Ok(Self::new(code, reason).unwrap())
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let class = (self.code / 100) as u32;
        let number = (self.code % 100) as u32;
        let value = (class << 8) | number;
        let mut writer = Cursor::new(Vec::new());
        track_try!(writer.write_u32be(value));
        track_try!(writer.write_all(self.reason_phrase.as_bytes()));
        Ok(writer.into_inner())
    }
}

/// `UNKNOWN-ATTRIBUTES` attribute.
///
/// See [RFC 5389 -- 15.9. UNKNOWN-ATTRIBUTES]
/// (https://tools.ietf.org/html/rfc5389#section-15.9) about this attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnknownAttributes {
    unknowns: Vec<Type>,
}
impl UnknownAttributes {
    /// Makes a new `UnknownAttributes` instance.
    pub fn new(unknowns: Vec<Type>) -> Self {
        UnknownAttributes { unknowns: unknowns }
    }

    /// Returns the unknown attribute types of this instance.
    pub fn unknowns(&self) -> &[Type] {
        &self.unknowns
    }
}
impl Attribute for UnknownAttributes {
    fn get_type(&self) -> Type {
        Type::new(TYPE_UNKNOWN_ATTRIBUTES)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_UNKNOWN_ATTRIBUTES,
                         ErrorKind::Failed);
        let count = attr.value().len() / 2;
        let mut unknowns = Vec::with_capacity(count);
        let mut reader = &mut attr.value();
        for _ in 0..count {
            let t = Type::new(track_try!(reader.read_u16be()));
            unknowns.push(t);
        }
        Ok(Self::new(unknowns))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let mut writer = Cursor::new(Vec::new());
        for u in self.unknowns.iter() {
            track_try!(writer.write_u16be(u.as_u16()))
        }
        Ok(writer.into_inner())
    }
}

/// `REALM` attribute.
///
/// See [RFC 5389 -- 15.7. REALM]
/// (https://tools.ietf.org/html/rfc5389#section-15.7) about this attribute.
///
/// # TODO
///
/// - Support SASLprep [RFC 4013]
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Realm {
    text: String,
}
impl Realm {
    /// Makes a new `Realm` instance.
    ///
    /// The length of `text` must be less than `128` characters.
    /// If it is too long, this will return `None`.
    pub fn new(text: String) -> Option<Self> {
        if text.chars().count() < 128 {
            Some(Realm { text: text })
        } else {
            None
        }
    }

    /// Returns the text of this instance.
    pub fn text(&self) -> &str {
        &self.text
    }
}
impl Attribute for Realm {
    fn get_type(&self) -> Type {
        Type::new(TYPE_REALM)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(), TYPE_REALM, ErrorKind::Failed);
        let mut reader = &mut attr.value();
        let text = track_try!(reader.read_all_string());
        track_assert!(text.chars().count() < 128, ErrorKind::Other);
        Ok(Self::new(text).unwrap())
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        Ok(Vec::from(self.text.as_bytes()))
    }
}

/// `NONCE` attribute.
///
/// See [RFC 5389 -- 15.8. NONCE]
/// (https://tools.ietf.org/html/rfc5389#section-15.8) about this attribute.
///
/// # TODO
///
/// - Support [RFC 3261] and [RFC 2617]
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nonce {
    value: String,
}
impl Nonce {
    /// Makes a new `Nonce` instance.
    ///
    /// The length of `value` must be less than `128` characters.
    /// If it is too long, this will return `None`.
    pub fn new(value: String) -> Option<Self> {
        if value.chars().count() < 128 {
            Some(Nonce { value: value })
        } else {
            None
        }
    }

    /// Returns the value of this instance.
    pub fn value(&self) -> &str {
        &self.value
    }
}
impl Attribute for Nonce {
    fn get_type(&self) -> Type {
        Type::new(TYPE_NONCE)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(), TYPE_NONCE, ErrorKind::Failed);
        let mut reader = &mut attr.value();
        let value = track_try!(reader.read_all_string());
        track_assert!(value.chars().count() < 128, ErrorKind::Other);
        Ok(Self::new(value).unwrap())
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        Ok(Vec::from(self.value.as_bytes()))
    }
}

/// `FINGERPRINT` attribute.
///
/// See [RFC 5389 -- 15.5. FINGERPRINT]
/// (https://tools.ietf.org/html/rfc5389#section-15.5) about this attribute.
///
/// # TODO
///
/// - Check CRC
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    crc32: u32,
}
impl Fingerprint {
    /// Makes a new `Fingerprint` instance.
    pub fn new(crc32: u32) -> Self {
        Fingerprint { crc32: crc32 }
    }

    /// Returns the crc32 value of this instance.
    pub fn crc32(&self) -> u32 {
        self.crc32
    }
}
impl Attribute for Fingerprint {
    fn get_type(&self) -> Type {
        Type::new(TYPE_FINGERPRINT)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_FINGERPRINT,
                         ErrorKind::Failed);
        track_assert_eq!(attr.value().len(), 4, ErrorKind::Other);
        let mut reader = &mut attr.value();
        let crc32 = track_try!(reader.read_u32be());
        Ok(Self::new(crc32))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let mut buf = vec![0; 4];
        track_try!((&mut buf[..]).write_u32be(self.crc32));
        Ok(buf)
    }
}

/// `SOFTWARE` attribute.
///
/// See [RFC 5389 -- 15.10. SOFTWARE]
/// (https://tools.ietf.org/html/rfc5389#section-15.10) about this attribute.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Software {
    description: String,
}
impl Software {
    /// Makes a new `Software` instance.
    ///
    /// The length of `description` must be less than `128` characters.
    /// If it is too long, this will return `None`.
    pub fn new(description: String) -> Option<Self> {
        if description.chars().count() < 128 {
            Some(Software { description: description })
        } else {
            None
        }
    }

    /// Returns the description of this instance.
    pub fn description(&self) -> &str {
        &self.description
    }
}
impl Attribute for Software {
    fn get_type(&self) -> Type {
        Type::new(TYPE_SOFTWARE)
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(), TYPE_SOFTWARE, ErrorKind::Failed);
        let mut reader = &mut attr.value();
        let description = track_try!(reader.read_all_string());
        track_assert!(description.chars().count() < 128, ErrorKind::Other);
        Ok(Self::new(description).unwrap())
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        Ok(Vec::from(self.description.as_bytes()))
    }
}

/// `XOR-MAPPED-ADDRESS` attribute.
///
/// See [RFC 5389 -- 15.2. XOR-MAPPED-ADDRESS]
/// (https://tools.ietf.org/html/rfc5389#section-15.2) about this attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XorMappedAddress(SocketAddr);
impl XorMappedAddress {
    /// Makes a new `XorMappedAddress` instance.
    pub fn new(addr: SocketAddr) -> Self {
        XorMappedAddress(addr)
    }

    /// Returns the address of this instance.
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
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_XOR_MAPPED_ADDRESS,
                         ErrorKind::Failed);
        let xor_addr = track_try!(read_socket_addr(&mut attr.value()));
        let addr = Self::xor_addr(xor_addr);
        Ok(Self::new(addr))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let xor_addr = Self::xor_addr(self.0);
        let mut buf = Vec::new();
        track_try!(write_socket_addr(&mut buf, xor_addr));
        Ok(buf)
    }
}

fn read_socket_addr<R: Read>(reader: &mut R) -> Result<SocketAddr> {
    let _ = track_try!(reader.read_u8());
    let family = track_try!(reader.read_u8());
    let port = track_try!(reader.read_u16be());
    let ip = match family {
        1 => {
            let ip = track_try!(reader.read_u32be());
            IpAddr::V4(From::from(ip))
        }
        2 => {
            let mut octets = [0; 16];
            track_try!(reader.read_exact(&mut octets[..]));
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
    track_try!(writer.write_u8(0));
    match addr.ip() {
        IpAddr::V4(ip) => {
            track_try!(writer.write_u8(1));
            track_try!(writer.write_u16be(addr.port()));
            track_try!(writer.write_all(&ip.octets()));
        }
        IpAddr::V6(ip) => {
            track_try!(writer.write_u8(2));
            track_try!(writer.write_u16be(addr.port()));
            track_try!(writer.write_all(&ip.octets()));
        }
    }
    Ok(())
}
