//! STUN attribute related components.
use std::io::{Read, Write};
use handy_async::sync_io::{ReadExt, WriteExt};

use Result;
use message::RawMessage;

/// STUN attribute.
///
/// > Attribute:  The STUN term for a Type-Length-Value (TLV) object that
/// > can be added to a STUN message. Attributes are divided into two
/// > types: comprehension-required and comprehension-optional. STUN
/// > agents can safely ignore comprehension-optional attributes they
/// > don't understand, but cannot successfully process a message if it
/// > contains comprehension-required attributes that are not
/// > understood.
/// >
/// > [RFC 5389 -- 5. Definitions](https://tools.ietf.org/html/rfc5389#section-5)
pub trait Attribute: Sized {
    /// Returns the attribute type of this instance.
    fn get_type(&self) -> Type;

    /// Tries to convert from `RawAttribute`.
    ///
    /// The `message` is a `RawMessage` instance which contains `attr`.
    /// The attributes contained in `message` are those that precede `attr`.
    fn try_from_raw(attr: &RawAttribute, message: &RawMessage) -> Result<Self>;

    /// Tries to convert to `RawAttribute`.
    ///
    /// The resulting attribute will be added at the tail of the attribute of the `message`.
    fn try_to_raw(&self, message: &RawMessage) -> Result<RawAttribute> {
        self.encode_value(message).map(|value| RawAttribute::new(self.get_type(), value))
    }

    /// Tries to encode the value of this attribute to bytes.
    fn encode_value(&self, message: &RawMessage) -> Result<Vec<u8>>;
}

/// Attribute type.
///
/// > Attributes are divided into two
/// > types: comprehension-required and comprehension-optional. STUN
/// > agents can safely ignore comprehension-optional attributes they
/// > don't understand, but cannot successfully process a message if it
/// > contains comprehension-required attributes that are not
/// > understood.
/// >
/// > [RFC 5389 -- 5. Definitions](https://tools.ietf.org/html/rfc5389#section-5)
/// >
/// > ---
/// >
/// > A STUN Attribute type is a hex number in the range 0x0000 - 0xFFFF.
/// > STUN attribute types in the range 0x0000 - 0x7FFF are considered
/// > comprehension-required; STUN attribute types in the range 0x8000 -
/// > 0xFFFF are considered comprehension-optional.
/// >
/// > [RFC 5389 -- 18.2. STUN Attribute Registry]
/// > (https://tools.ietf.org/html/rfc5389#section-18.2)
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Type(u16);
impl Type {
    /// Makes a new `Type` instance which corresponding to `codepoint`.
    pub fn new(codepoint: u16) -> Self {
        Type(codepoint)
    }

    /// Returns the attribute codepoint corresponding this instance.
    pub fn as_u16(&self) -> u16 {
        self.0
    }

    /// Returns `true` if this is a comprehension-required type.
    pub fn is_comprehension_required(&self) -> bool {
        self.0 < 0x8000
    }

    /// Returns `true` if this is a comprehension-optional type.
    pub fn is_comprehension_optional(&self) -> bool {
        !self.is_comprehension_required()
    }
}
impl From<u16> for Type {
    fn from(f: u16) -> Self {
        Self::new(f)
    }
}

/// The raw representation of a STUN attribute.
///
/// It is possible to perform conversion between an instance and
/// the corresponding bytes without loss of information
/// (including padding bytes).
///
/// # NOTE: Binary Format of STUN Attributes
///
/// > After the STUN header are zero or more attributes. Each attribute
/// > MUST be TLV encoded, with a 16-bit type, 16-bit length, and value.
/// > Each STUN attribute MUST end on a 32-bit boundary. As mentioned
/// > above, all fields in an attribute are transmitted most significant
/// > bit first.
/// >
/// > ```test
/// >  0                   1                   2                   3
/// >  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// > +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// > |         Type                  |            Length             |
/// > +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// > |                         Value (variable)                ....
/// > +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// >
/// >               Figure 4: Format of STUN Attributes
/// > ```
/// >
/// > The value in the length field MUST contain the length of the Value
/// > part of the attribute, prior to padding, measured in bytes. Since
/// > STUN aligns attributes on 32-bit boundaries, attributes whose content
/// > is not a multiple of 4 bytes are padded with 1, 2, or 3 bytes of
/// > padding so that its value contains a multiple of 4 bytes. The
/// > padding bits are ignored, and may be any value.
/// >
/// > [RFC 5389 -- 15. STUN Attributes](https://tools.ietf.org/html/rfc5389#section-15)
#[derive(Debug, Clone)]
pub struct RawAttribute {
    attr_type: Type,
    value: Vec<u8>,
    padding: [u8; 4],
}
impl RawAttribute {
    /// Makes a new `RawAttribute` instance.
    pub fn new(attr_type: Type, value: Vec<u8>) -> Self {
        assert!(value.len() < 0x10000);
        RawAttribute {
            attr_type: attr_type,
            value: value,
            padding: [0; 4],
        }
    }

    /// Returns the value bytes of this attribute.
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    /// Returns the padding bytes of this attribute.
    pub fn padding(&self) -> &[u8] {
        let padding_len = (4 - self.value.len() % 4) % 4;
        &self.padding[..padding_len]
    }

    /// Reads bytes from `reader` and decodes it to a `RawAttribute` instance.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let attr_type = track_try!(reader.read_u16be());
        let attr_type = Type::new(attr_type);
        let value_len = track_try!(reader.read_u16be()) as u64;
        let value = track_try!(reader.take(value_len).read_all_bytes());
        let mut padding = [0; 4];
        let padding_len = ((4 - value_len % 4) % 4) as usize;
        track_try!(reader.read_exact(&mut padding[..padding_len]));
        Ok(RawAttribute {
            attr_type: attr_type,
            value: value,
            padding: padding,
        })
    }

    /// Writes the binary format of this attribute to `writer`.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        track_try!(writer.write_u16be(self.attr_type.as_u16()));
        track_try!(writer.write_u16be(self.value.len() as u16));
        track_try!(writer.write_all(&self.value));
        track_try!(writer.write_all(self.padding()));
        Ok(())
    }
}
impl Attribute for RawAttribute {
    fn get_type(&self) -> Type {
        self.attr_type
    }
    fn try_from_raw(attr: &RawAttribute, _message: &RawMessage) -> Result<Self> {
        Ok(attr.clone())
    }
    fn encode_value(&self, _message: &RawMessage) -> Result<Vec<u8>> {
        Ok(self.value.clone())
    }
}
