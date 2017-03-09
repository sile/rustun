//! Individual Definition of the attributes that are defined in [RFC 5389]
//! (https://tools.ietf.org/html/rfc5389).
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{Write, Cursor};
use std::net::SocketAddr;
use crc::crc32;
use md5;
use hmacsha1;
use handy_async::sync_io::{ReadExt, WriteExt};

use {Result, Attribute, ErrorKind};
use message::{RawMessage, Message};
use attribute::{Type, RawAttribute};
use types::{SocketAddrValue, TryAsRef};
use rfc5389::errors;

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
                         ErrorKind::Unsupported);
        let addr = track_try!(SocketAddrValue::read_from(&mut attr.value()));
        Ok(Self::new(addr.address()))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        track_try!(SocketAddrValue::new(self.0).write_to(&mut buf));
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
                         ErrorKind::Unsupported);
        let addr = track_try!(SocketAddrValue::read_from(&mut attr.value()));
        Ok(Self::new(addr.address()))
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        track_try!(SocketAddrValue::new(self.0).write_to(&mut buf));
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
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_USERNAME,
                         ErrorKind::Unsupported);
        let name = track_try!((&mut attr.value()).read_all_string());
        track_assert!(name.len() < 513, ErrorKind::Invalid);
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
/// - Support SASLprep
///
#[derive(Clone)]
pub struct MessageIntegrity {
    hmac_sha1: [u8; 20],
    preceding: RawMessage,
}
impl MessageIntegrity {
    /// Makes a new `MessageIntegrity` instance for short-term credentials.
    pub fn new_short_term_credential<M>(message: &M, password: &str) -> Result<Self>
        where M: Message
    {
        let key = password.as_bytes();
        let preceding = track_try!(message.try_to_raw());
        let hmac = Self::calc_hmac_sha1(key, &preceding);
        Ok(MessageIntegrity {
            hmac_sha1: hmac,
            preceding: preceding,
        })
    }

    /// Makes a new `MessageIntegrity` instance for long-term credentials.
    pub fn new_long_term_credential<M>(message: &M, password: &str) -> Result<Self>
        where M: Message,
              M::Attribute: TryAsRef<Username> + TryAsRef<Realm>
    {
        let username = track_try!(message.get_attribute::<Username>()
            .ok_or_else(|| ErrorKind::ErrorCode(errors::BadRequest.into())));
        let realm = track_try!(message.get_attribute::<Realm>()
            .ok_or_else(|| ErrorKind::ErrorCode(errors::BadRequest.into())));
        let key = md5::compute(format!("{}:{}:{}", username.name(), realm.text(), password)
            .as_bytes());
        let preceding = track_try!(message.try_to_raw());
        let hmac = Self::calc_hmac_sha1(&key.0[..], &preceding);
        Ok(MessageIntegrity {
            hmac_sha1: hmac,
            preceding: preceding,
        })
    }

    /// Checks whether this has the valid short-term credential for `password`.
    pub fn check_short_term_credential(&self, password: &str) -> Result<()> {
        let key = password.as_bytes();
        let expected = Self::calc_hmac_sha1(key, &self.preceding);
        track_assert_eq!(self.hmac_sha1,
                         expected,
                         ErrorKind::ErrorCode(errors::Unauthorized.into()));
        Ok(())
    }

    /// Checks whether this has the valid long-term credential for `password`.
    pub fn check_long_term_credential(&self, password: &str) -> Result<()> {
        let username = if let Some(a) = self.preceding
            .attributes()
            .into_iter()
            .find(|a| a.get_type().as_u16() == TYPE_USERNAME) {
            track_try!(Username::try_from_raw(a, &self.preceding))
        } else {
            track_panic!(ErrorKind::ErrorCode(errors::BadRequest.into()));
        };
        let realm = if let Some(a) = self.preceding
            .attributes()
            .into_iter()
            .find(|a| a.get_type().as_u16() == TYPE_REALM) {
            track_try!(Realm::try_from_raw(a, &self.preceding))
        } else {
            track_panic!(ErrorKind::ErrorCode(errors::BadRequest.into()));
        };
        let key = md5::compute(format!("{}:{}:{}", username.name(), realm.text(), password)
            .as_bytes());
        let expected = Self::calc_hmac_sha1(&key.0[..], &self.preceding);
        track_assert_eq!(self.hmac_sha1,
                         expected,
                         ErrorKind::ErrorCode(errors::Unauthorized.into()));
        Ok(())
    }

    /// Returns the HMAC-SHA1 of this instance.
    pub fn hmac_sha1(&self) -> [u8; 20] {
        self.hmac_sha1
    }

    fn calc_hmac_sha1(key: &[u8], preceding: &RawMessage) -> [u8; 20] {
        let mut bytes = preceding.to_bytes();
        let adjusted_len = bytes.len() - 20 /*msg header*/+ 4 /*attr header*/ + 20 /*hmac*/;
        (&mut bytes[2..4]).write_u16be(adjusted_len as u16).expect("must succeed");
        hmacsha1::hmac_sha1(key, &bytes[..])
    }
}
impl Attribute for MessageIntegrity {
    fn get_type(&self) -> Type {
        Type::new(TYPE_MESSAGE_INTEGRITY)
    }
    fn try_from_raw(attr: &RawAttribute, message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_MESSAGE_INTEGRITY,
                         ErrorKind::Unsupported);
        track_assert_eq!(attr.value().len(), 20, ErrorKind::Invalid);
        let mut hmac_sha1 = [0; 20];
        (&mut hmac_sha1[..]).copy_from_slice(attr.value());
        Ok(MessageIntegrity {
            hmac_sha1: hmac_sha1,
            preceding: message.clone(),
        })
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        Ok(Vec::from(&self.hmac_sha1[..]))
    }
}
impl Hash for MessageIntegrity {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.hmac_sha1.hash(hasher);
    }
}
impl PartialEq for MessageIntegrity {
    fn eq(&self, other: &Self) -> bool {
        self.hmac_sha1 == other.hmac_sha1
    }
}
impl Eq for MessageIntegrity {}
impl fmt::Debug for MessageIntegrity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "MessageIntegrity {{ hmac_sha1: {:?}, .. }}",
               self.hmac_sha1)
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
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_ERROR_CODE,
                         ErrorKind::Unsupported);
        let mut reader = &mut attr.value();
        let value = track_try!(reader.read_u32be());
        let class = (value >> 8) & 0b111;
        let number = value & 0b11111111;
        track_assert!(3 <= class && class < 6, ErrorKind::Invalid);
        track_assert!(number < 100, ErrorKind::Invalid);

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
                         ErrorKind::Unsupported);
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
        track_assert_eq!(attr.get_type().as_u16(), TYPE_REALM, ErrorKind::Unsupported);
        let mut reader = &mut attr.value();
        let text = track_try!(reader.read_all_string());
        track_assert!(text.chars().count() < 128, ErrorKind::Invalid);
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
        track_assert_eq!(attr.get_type().as_u16(), TYPE_NONCE, ErrorKind::Unsupported);
        let mut reader = &mut attr.value();
        let value = track_try!(reader.read_all_string());
        track_assert!(value.chars().count() < 128, ErrorKind::Invalid);
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    crc32: u32,
}
impl Fingerprint {
    /// Makes a new `Fingerprint` instance.
    pub fn new() -> Self {
        Fingerprint { crc32: 0 }
    }

    /// Returns the crc32 value of this instance.
    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    /// Calculates the CRC-32 value of `message` and returns a `Fingerprint` instance containing it.
    pub fn from_message(message: &RawMessage) -> Self {
        let mut bytes = message.to_bytes();
        let final_len = bytes.len() - 20 + 8; // Adds `Fingerprint` attribute length
        (&mut bytes[2..4]).write_u16be(final_len as u16).expect("must succeed");
        let crc32 = crc32::checksum_ieee(&bytes[..]) ^ 0x5354554e;
        Fingerprint { crc32: crc32 }
    }
}
impl Attribute for Fingerprint {
    fn get_type(&self) -> Type {
        Type::new(TYPE_FINGERPRINT)
    }
    fn try_from_raw(attr: &RawAttribute, message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_FINGERPRINT,
                         ErrorKind::Unsupported);
        track_assert_eq!(attr.value().len(), 4, ErrorKind::Invalid);
        let mut reader = &mut attr.value();
        let crc32 = track_try!(reader.read_u32be());

        let expected = Self::from_message(message);
        track_assert_eq!(crc32, expected.crc32, ErrorKind::Invalid);
        Ok(expected)
    }
    fn encode_value(&self, message: &RawMessage) -> Result<Vec<u8>> {
        let fingerprint = Self::from_message(message);
        let mut buf = vec![0; 4];
        track_try!((&mut buf[..]).write_u32be(fingerprint.crc32));
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
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_SOFTWARE,
                         ErrorKind::Unsupported);
        let mut reader = &mut attr.value();
        let description = track_try!(reader.read_all_string());
        track_assert!(description.chars().count() < 128, ErrorKind::Invalid);
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
}
impl Attribute for XorMappedAddress {
    fn get_type(&self) -> Type {
        Type::new(TYPE_XOR_MAPPED_ADDRESS)
    }
    fn try_from_raw(attr: &RawAttribute, message: &RawMessage) -> Result<Self> {
        track_assert_eq!(attr.get_type().as_u16(),
                         TYPE_XOR_MAPPED_ADDRESS,
                         ErrorKind::Unsupported);
        let xor_addr = track_try!(SocketAddrValue::read_from(&mut attr.value()));
        let addr = xor_addr.xor(message.transaction_id()).address();
        Ok(Self::new(addr))
    }
    fn encode_value(&self, message: &RawMessage) -> Result<Vec<u8>> {
        let xor_addr = SocketAddrValue::new(self.0).xor(message.transaction_id());
        let mut buf = Vec::new();
        track_try!(xor_addr.write_to(&mut buf));
        Ok(buf)
    }
}

#[cfg(test)]
mod test {
    use message::{self, RawMessage, Message};
    use rfc5389::{Method, Attribute};
    use super::*;

    type Request = message::Request<Method, Attribute>;
    type Response = message::Response<Method, Attribute>;

    #[test]
    fn rfc5769_2_1_sample_request() {
        let input = [0x00, 0x01, 0x00, 0x58, 0x21, 0x12, 0xa4, 0x42, 0xb7, 0xe7, 0xa7, 0x01, 0xbc,
                     0x34, 0xd6, 0x86, 0xfa, 0x87, 0xdf, 0xae, 0x80, 0x22, 0x00, 0x10, 0x53, 0x54,
                     0x55, 0x4e, 0x20, 0x74, 0x65, 0x73, 0x74, 0x20, 0x63, 0x6c, 0x69, 0x65, 0x6e,
                     0x74, 0x00, 0x24, 0x00, 0x04, 0x6e, 0x00, 0x01, 0xff, 0x80, 0x29, 0x00, 0x08,
                     0x93, 0x2f, 0xf9, 0xb1, 0x51, 0x26, 0x3b, 0x36, 0x00, 0x06, 0x00, 0x09, 0x65,
                     0x76, 0x74, 0x6a, 0x3a, 0x68, 0x36, 0x76, 0x59, 0x20, 0x20, 0x20, 0x00, 0x08,
                     0x00, 0x14, 0x9a, 0xea, 0xa7, 0x0c, 0xbf, 0xd8, 0xcb, 0x56, 0x78, 0x1e, 0xf2,
                     0xb5, 0xb2, 0xd3, 0xf2, 0x49, 0xc1, 0xb5, 0x71, 0xa2, 0x80, 0x28, 0x00, 0x04,
                     0xe5, 0x7a, 0x3b, 0xcf];
        let mut message = RawMessage::read_from(&mut &input[..]).unwrap();

        // TEST: `MessageIntegrity`
        let request: Request = message.clone().try_into_request().unwrap();
        let password = "VOkJxbRl1RmTxUk/WvJxBt";
        request.get_attribute::<MessageIntegrity>()
            .unwrap()
            .check_short_term_credential(password)
            .unwrap();

        // TEST: `Fingerprint`
        message.pop_attribute(); // Removes `Fingerprint` attribute
        let fingerprint = Fingerprint::from_message(&message);
        assert_eq!(fingerprint.crc32(), 0xe57a3bcf);
    }

    #[test]
    fn rfc5769_2_2_sample_ipv4_response() {
        let input = [0x01, 0x01, 0x00, 0x3c, 0x21, 0x12, 0xa4, 0x42, 0xb7, 0xe7, 0xa7, 0x01, 0xbc,
                     0x34, 0xd6, 0x86, 0xfa, 0x87, 0xdf, 0xae, 0x80, 0x22, 0x00, 0x0b, 0x74, 0x65,
                     0x73, 0x74, 0x20, 0x76, 0x65, 0x63, 0x74, 0x6f, 0x72, 0x20, 0x00, 0x20, 0x00,
                     0x08, 0x00, 0x01, 0xa1, 0x47, 0xe1, 0x12, 0xa6, 0x43, 0x00, 0x08, 0x00, 0x14,
                     0x2b, 0x91, 0xf5, 0x99, 0xfd, 0x9e, 0x90, 0xc3, 0x8c, 0x74, 0x89, 0xf9, 0x2a,
                     0xf9, 0xba, 0x53, 0xf0, 0x6b, 0xe7, 0xd7, 0x80, 0x28, 0x00, 0x04, 0xc0, 0x7d,
                     0x4c, 0x96];
        let mut message = RawMessage::read_from(&mut &input[..]).unwrap();

        // TEST: `MessageIntegrity`
        let response: Response = message.clone().try_into_response().unwrap();
        let password = "VOkJxbRl1RmTxUk/WvJxBt";
        response.get_attribute::<MessageIntegrity>()
            .unwrap()
            .check_short_term_credential(password)
            .unwrap();

        // TEST: `XorMappedAddress` (IPv4)
        assert_eq!(response.get_attribute::<XorMappedAddress>().unwrap().address(),
                   "192.0.2.1:32853".parse().unwrap());

        // TEST: `Fingerprint`
        message.pop_attribute(); // Removes `Fingerprint` attribute
        let fingerprint = Fingerprint::from_message(&message);
        assert_eq!(fingerprint.crc32(), 0xc07d4c96);
    }

    #[test]
    fn rfc5769_2_3_sample_ipv6_response() {
        let input = [0x01, 0x01, 0x00, 0x48, 0x21, 0x12, 0xa4, 0x42, 0xb7, 0xe7, 0xa7, 0x01, 0xbc,
                     0x34, 0xd6, 0x86, 0xfa, 0x87, 0xdf, 0xae, 0x80, 0x22, 0x00, 0x0b, 0x74, 0x65,
                     0x73, 0x74, 0x20, 0x76, 0x65, 0x63, 0x74, 0x6f, 0x72, 0x20, 0x00, 0x20, 0x00,
                     0x14, 0x00, 0x02, 0xa1, 0x47, 0x01, 0x13, 0xa9, 0xfa, 0xa5, 0xd3, 0xf1, 0x79,
                     0xbc, 0x25, 0xf4, 0xb5, 0xbe, 0xd2, 0xb9, 0xd9, 0x00, 0x08, 0x00, 0x14, 0xa3,
                     0x82, 0x95, 0x4e, 0x4b, 0xe6, 0x7b, 0xf1, 0x17, 0x84, 0xc9, 0x7c, 0x82, 0x92,
                     0xc2, 0x75, 0xbf, 0xe3, 0xed, 0x41, 0x80, 0x28, 0x00, 0x04, 0xc8, 0xfb, 0x0b,
                     0x4c];
        let mut message = RawMessage::read_from(&mut &input[..]).unwrap();

        // TEST: `MessageIntegrity`
        let response: Response = message.clone().try_into_response().unwrap();
        let password = "VOkJxbRl1RmTxUk/WvJxBt";
        response.get_attribute::<MessageIntegrity>()
            .unwrap()
            .check_short_term_credential(password)
            .unwrap();

        // TEST: `XorMappedAddress` (IPv6)
        assert_eq!(response.get_attribute::<XorMappedAddress>().unwrap().address(),
                   "[2001:db8:1234:5678:11:2233:4455:6677]:32853".parse().unwrap());

        // TEST: `Fingerprint`
        message.pop_attribute(); // Removes `Fingerprint` attribute
        let fingerprint = Fingerprint::from_message(&message);
        assert_eq!(fingerprint.crc32(), 0xc8fb0b4c);
    }

    #[test]
    fn rfc5769_2_4_sample_request_with_long_term_authentication() {
        let input = [0x00, 0x01, 0x00, 0x60, 0x21, 0x12, 0xa4, 0x42, 0x78, 0xad, 0x34, 0x33, 0xc6,
                     0xad, 0x72, 0xc0, 0x29, 0xda, 0x41, 0x2e, 0x00, 0x06, 0x00, 0x12, 0xe3, 0x83,
                     0x9e, 0xe3, 0x83, 0x88, 0xe3, 0x83, 0xaa, 0xe3, 0x83, 0x83, 0xe3, 0x82, 0xaf,
                     0xe3, 0x82, 0xb9, 0x00, 0x00, 0x00, 0x15, 0x00, 0x1c, 0x66, 0x2f, 0x2f, 0x34,
                     0x39, 0x39, 0x6b, 0x39, 0x35, 0x34, 0x64, 0x36, 0x4f, 0x4c, 0x33, 0x34, 0x6f,
                     0x4c, 0x39, 0x46, 0x53, 0x54, 0x76, 0x79, 0x36, 0x34, 0x73, 0x41, 0x00, 0x14,
                     0x00, 0x0b, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67,
                     0x00, 0x00, 0x08, 0x00, 0x14, 0xf6, 0x70, 0x24, 0x65, 0x6d, 0xd6, 0x4a, 0x3e,
                     0x02, 0xb8, 0xe0, 0x71, 0x2e, 0x85, 0xc9, 0xa2, 0x8c, 0xa8, 0x96, 0x66];
        let message = RawMessage::read_from(&mut &input[..]).unwrap();

        // TEST: `MessageIntegrity`
        let request: Request = message.clone().try_into_request().unwrap();
        let password = "TheMatrIX"; // TODO: Test before SASLprep version
        request.get_attribute::<MessageIntegrity>()
            .unwrap()
            .check_long_term_credential(password)
            .unwrap();
    }
}
