use bytecodec::io::IoEncodeExt;
use bytecodec::Encode;
use handy_async::sync_io::{ReadExt, WriteExt};
use std::io::{self, Read, Write};
use std::mem;
use trackable::error::ErrorKindExt;

use attribute::RawAttribute;
use constants::MAGIC_COOKIE;
use message::{ErrorResponse, Indication, Message, Request, Response, SuccessResponse};
use types::{TransactionId, U12};
use {Attribute, ErrorKind, Method, Result};

/// The raw representation of a STUN message.
///
/// It is possible to perform conversion between an instance and
/// the corresponding bytes without loss of information.
///
/// # NOTE: Binary Format of STUN Messages
///
/// > STUN messages are encoded in binary using network-oriented format
/// > (most significant byte or octet first, also commonly known as big-
/// > endian).  The transmission order is described in detail in Appendix B
/// > of RFC 791 [RFC0791].  Unless otherwise noted, numeric constants are
/// > in decimal (base 10).
/// >
/// > All STUN messages MUST start with a 20-byte header followed by zero
/// > or more Attributes.  The STUN header contains a STUN message type,
/// > magic cookie, transaction ID, and message length.
/// >
/// > ```text
/// >  0                   1                   2                   3
/// >  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// > +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// > |0 0|     STUN Message Type     |         Message Length        |
/// > +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// > |                         Magic Cookie                          |
/// > +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// > |                                                               |
/// > |                     Transaction ID (96 bits)                  |
/// > |                                                               |
/// > +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// >
/// >             Figure 2: Format of STUN Message Header
/// > ```
/// >
/// > The most significant 2 bits of every STUN message MUST be zeroes.
/// > This can be used to differentiate STUN packets from other protocols
/// > when STUN is multiplexed with other protocols on the same port.
/// >
/// > The message type defines the message class (request, success
/// > response, failure response, or indication) and the message method
/// > (the primary function) of the STUN message.  Although there are four
/// > message classes, there are only two types of transactions in STUN:
/// > request/response transactions (which consist of a request message and
/// > a response message) and indication transactions (which consist of a
/// > single indication message).  Response classes are split into error
/// > and success responses to aid in quickly processing the STUN message.
/// >
/// > The message type field is decomposed further into the following structure:
/// >
/// > ```text
/// >  0                 1
/// >  2  3  4 5 6 7 8 9 0 1 2 3 4 5
/// > +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
/// > |M |M |M|M|M|C|M|M|M|C|M|M|M|M|
/// > |11|10|9|8|7|1|6|5|4|0|3|2|1|0|
/// > +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
/// >
/// > Figure 3: Format of STUN Message Type Field
/// > ```
/// >
/// > Here the bits in the message type field are shown as most significant
/// > (M11) through least significant (M0).  M11 through M0 represent a 12-
/// > bit encoding of the method.  C1 and C0 represent a 2-bit encoding of
/// > the class.  A class of 0b00 is a request, a class of 0b01 is an
/// > indication, a class of 0b10 is a success response, and a class of
/// > 0b11 is an error response.  This specification defines a single
/// > method, Binding.  The method and class are orthogonal, so that for
/// > each method, a request, success response, error response, and
/// > indication are possible for that method.  Extensions defining new
/// > methods MUST indicate which classes are permitted for that method.
/// >
/// > For example, a Binding request has class=0b00 (request) and
/// > method=0b000000000001 (Binding) and is encoded into the first 16 bits
/// > as 0x0001.  A Binding response has class=0b10 (success response) and
/// > method=0b000000000001, and is encoded into the first 16 bits as 0x0101.
/// >
/// > > Note: This unfortunate encoding is due to assignment of values in
/// > > [RFC3489] that did not consider encoding Indications, Success, and
/// > > Errors using bit fields.
/// >
/// > The magic cookie field MUST contain the fixed value 0x2112A442 in
/// > network byte order.  In RFC 3489 [RFC3489], this field was part of
/// > the transaction ID; placing the magic cookie in this location allows
/// > a server to detect if the client will understand certain attributes
/// > that were added in this revised specification.  In addition, it aids
/// > in distinguishing STUN packets from packets of other protocols when
/// > STUN is multiplexed with those other protocols on the same port.
/// >
/// > The transaction ID is a 96-bit identifier, used to uniquely identify
/// > STUN transactions.  For request/response transactions, the
/// > transaction ID is chosen by the STUN client for the request and
/// > echoed by the server in the response.  For indications, it is chosen
/// > by the agent sending the indication.  It primarily serves to
/// > correlate requests with responses, though it also plays a small role
/// > in helping to prevent certain types of attacks.  The server also uses
/// > the transaction ID as a key to identify each transaction uniquely
/// > across all clients.  As such, the transaction ID MUST be uniformly
/// > and randomly chosen from the interval 0 .. 2**96-1, and SHOULD be
/// > cryptographically random.  Resends of the same request reuse the same
/// > transaction ID, but the client MUST choose a new transaction ID for
/// > new transactions unless the new request is bit-wise identical to the
/// > previous request and sent from the same transport address to the same
/// > IP address.  Success and error responses MUST carry the same
/// > transaction ID as their corresponding request.  When an agent is
/// > acting as a STUN server and STUN client on the same port, the
/// > transaction IDs in requests sent by the agent have no relationship to
/// > the transaction IDs in requests received by the agent.
/// >
/// > The message length MUST contain the size, in bytes, of the message
/// > not including the 20-byte STUN header.  Since all STUN attributes are
/// > padded to a multiple of 4 bytes, the last 2 bits of this field are
/// > always zero.  This provides another way to distinguish STUN packets
/// > from packets of other protocols.
/// >
/// > Following the STUN fixed portion of the header are zero or more
/// > attributes.  Each attribute is TLV (Type-Length-Value) encoded.  The
/// > details of the encoding, and of the attributes themselves are given
/// > in Section 15.
/// >
/// > [RFC 5389 -- 6. STUN Message Structure](https://tools.ietf.org/html/rfc5389#section-6)
#[derive(Debug, Clone)]
pub struct RawMessage {
    class: Class,
    method: U12,
    transaction_id: TransactionId,
    attributes: Vec<RawAttribute>,
}
impl RawMessage {
    /// Makes a new `RawMessage` instance.
    pub fn new(
        class: Class,
        method: U12,
        transaction_id: TransactionId,
        attributes: Vec<RawAttribute>,
    ) -> Self {
        RawMessage {
            class: class,
            method: method,
            transaction_id: transaction_id,
            attributes: attributes,
        }
    }

    /// Returns the class of this message.
    pub fn class(&self) -> Class {
        self.class
    }

    /// Returns the method of this message.
    pub fn method(&self) -> U12 {
        self.method
    }

    /// Returns the transaction ID of this message.
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }

    /// Returns the attributes of this message.
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn push_attribute(&mut self, attribute: RawAttribute) {
        self.attributes.push(attribute);
    }

    /// Removes an attribute from the tail of the attributes of this message.
    pub fn pop_attribute(&mut self) -> Option<RawAttribute> {
        self.attributes.pop()
    }

    /// Converts this message to the corresponding binary format.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("must succeed");
        buf
    }

    /// Writes the binary format of this message to `writer`.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut temp_writer = io::Cursor::new(vec![0; 20]);
        let message_type = Type {
            class: self.class,
            method: self.method,
        };
        track_try!(temp_writer.write_u16be(message_type.as_u16()));
        track_try!(temp_writer.write_u16be(0)); // dummy length
        track_try!(temp_writer.write_u32be(MAGIC_COOKIE));
        track_try!(temp_writer.write_all(&self.transaction_id));

        let mut encoder = RawAttribute::encoder();
        for attr in self.attributes.iter() {
            track!(encoder.start_encoding(attr.clone()))?;
            track!(encoder.encode_all(&mut temp_writer))?;
        }

        let attrs_len = temp_writer.get_ref().len() - 20;
        track_assert!(
            attrs_len < 0x10000,
            ErrorKind::Invalid,
            "Too large message length: actual={}, limit=0xFFFF",
            attrs_len
        );
        temp_writer.set_position(2);
        track_try!(temp_writer.write_u16be(attrs_len as u16));

        let buf = temp_writer.into_inner();
        track_try!(writer.write_all(&buf));
        Ok(())
    }

    /// Reads the binary format of a message from `reader` and returns the resulting message.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let message_type = track_try!(reader.read_u16be());
        let message_type = track_try!(Type::from_u16(message_type));
        let message_len = track_try!(reader.read_u16be());
        track_assert!(
            message_len % 4 == 0,
            ErrorKind::Invalid,
            "Unexpected message length: {} % 4 != 0",
            message_len
        );
        let magic_cookie = track_try!(reader.read_u32be());
        track_assert!(
            magic_cookie == MAGIC_COOKIE,
            ErrorKind::Invalid,
            "Unexpected magic cookie: actual={}, expected={}",
            magic_cookie,
            MAGIC_COOKIE
        );;

        let mut transaction_id: [u8; 12] = [0; 12];
        track_try!(reader.read_exact(&mut transaction_id));

        let mut attrs = Vec::new();
        let mut reader = reader.take(message_len as u64);
        while reader.limit() > 0 {
            let attr = track_try!(RawAttribute::read_from(&mut reader));
            attrs.push(attr);
        }
        Ok(RawMessage::new(
            message_type.class,
            message_type.method,
            transaction_id,
            attrs,
        ))
    }

    /// Tries to convert into the corresponding request message.
    pub fn try_into_request<M: Method, A: Attribute>(self) -> Result<Request<M, A>> {
        track_assert_eq!(self.class, Class::Request, ErrorKind::Invalid);
        let (method, transaction_id, attrs) = track_try!(self.try_into());
        Ok(Request {
            method: method,
            transaction_id: transaction_id,
            attributes: attrs,
        })
    }

    /// Tries to convert into the corresponding indication message.
    pub fn try_into_indication<M: Method, A: Attribute>(self) -> Result<Indication<M, A>> {
        track_assert_eq!(self.class, Class::Indication, ErrorKind::Invalid);
        let (method, transaction_id, attrs) = track_try!(self.try_into());
        Ok(Indication {
            method: method,
            transaction_id: transaction_id,
            attributes: attrs,
        })
    }

    /// Tries to convert into the corresponding responses message.
    pub fn try_into_response<M: Method, A: Attribute>(self) -> Result<Response<M, A>> {
        let class = self.class;
        track_assert!(
            class == Class::SuccessResponse || class == Class::ErrorResponse,
            ErrorKind::Invalid
        );
        let (method, transaction_id, attrs) = track_try!(self.try_into());
        if class == Class::SuccessResponse {
            Ok(Ok(SuccessResponse {
                method: method,
                transaction_id: transaction_id,
                attributes: attrs,
            }))
        } else {
            Ok(Err(ErrorResponse {
                method: method,
                transaction_id: transaction_id,
                attributes: attrs,
            }))
        }
    }

    /// Tries to convert from `Request` to `RawMessage`.
    pub fn try_from_request<M: Method, A: Attribute>(from: Request<M, A>) -> Result<Self> {
        track_err!(Self::try_from(
            Class::Request,
            from.method(),
            *from.transaction_id(),
            from.attributes(),
        ))
    }

    /// Tries to convert from `Indication` to `RawMessage`.
    pub fn try_from_indication<M: Method, A: Attribute>(from: Indication<M, A>) -> Result<Self> {
        track_err!(Self::try_from(
            Class::Indication,
            from.method(),
            *from.transaction_id(),
            from.attributes(),
        ))
    }

    /// Tries to convert from `Response` to `RawMessage`.
    pub fn try_from_response<M: Method, A: Attribute>(from: Response<M, A>) -> Result<Self> {
        track_err!(match from {
            Ok(m) => Self::try_from(
                Class::SuccessResponse,
                m.method(),
                *m.transaction_id(),
                m.attributes(),
            ),
            Err(m) => Self::try_from(
                Class::ErrorResponse,
                m.method(),
                *m.transaction_id(),
                m.attributes(),
            ),
        })
    }

    fn try_from<M: Method, A: Attribute>(
        class: Class,
        method: &M,
        transaction_id: TransactionId,
        attributes: &[A],
    ) -> Result<Self> {
        let mut m = RawMessage::new(class, method.as_u12(), transaction_id, Vec::new());
        for attr in attributes {
            let raw_attr = track_try!(attr.try_to_raw(&m));
            m.attributes.push(raw_attr);
        }
        Ok(m)
    }
    fn try_into<M: Method, A: Attribute>(mut self) -> Result<(M, TransactionId, Vec<A>)> {
        let method = track_try!(M::from_u12(self.method).ok_or_else(|| {
            ErrorKind::Unsupported.cause(format!("Unknown method: {:?}", self.method))
        }));
        let attrs_len = self.attributes.len();
        let src_attrs = mem::replace(&mut self.attributes, Vec::with_capacity(attrs_len));
        let mut dst_attrs = Vec::with_capacity(attrs_len);
        for a in src_attrs {
            let raw = track_try!(A::try_from_raw(&a, &self));
            dst_attrs.push(raw);
            self.attributes.push(a);
        }
        Ok((method, self.transaction_id, dst_attrs))
    }
}
impl Message for RawMessage {
    type Method = U12;
    type Attribute = RawAttribute;
    fn get_class(&self) -> Class {
        self.class()
    }
    fn get_method(&self) -> &Self::Method {
        &self.method
    }
    fn get_transaction_id(&self) -> &TransactionId {
        self.transaction_id()
    }
    fn get_attributes(&self) -> &[Self::Attribute] {
        self.attributes()
    }
    fn try_to_raw(&self) -> Result<RawMessage> {
        Ok(self.clone())
    }
}

/// The class of a message.
///
/// > The class indicates whether this is a **request**, a **success response**,
/// > an **error response**, or an **indication**.
/// >
/// > [RFC 5389 -- 3. Overview of Operation](https://tools.ietf.org/html/rfc5389#section-3)
///
/// An instance of `Class` can be casted to the corresponding `u8` value.
///
/// ```
/// use rustun::message::Class;
///
/// assert_eq!(Class::Request as u8, 0b00);
/// assert_eq!(Class::Indication as u8, 0b01);
/// assert_eq!(Class::SuccessResponse as u8, 0b10);
/// assert_eq!(Class::ErrorResponse as u8, 0b11);
/// ```
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Class {
    Request = 0b00,
    Indication = 0b01,
    SuccessResponse = 0b10,
    ErrorResponse = 0b11,
}
impl Class {
    /// Returns a `Class` instance which is corresponding to `value`.
    ///
    /// > A class of `0b00` is a request, a class of `0b01` is an
    /// > indication, a class of `0b10` is a success response, and a class of
    /// > `0b11` is an error response.
    /// >
    /// > [RFC 5389 -- 6. STUN Message Structure](https://tools.ietf.org/html/rfc5389#section-6)
    ///
    /// If no such instance exists, this will return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustun::message::Class;
    ///
    /// assert_eq!(Class::from_u8(0), Some(Class::Request));
    /// assert_eq!(Class::from_u8(9), None);
    /// ```
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Class::Request),
            0b01 => Some(Class::Indication),
            0b10 => Some(Class::SuccessResponse),
            0b11 => Some(Class::ErrorResponse),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Type {
    pub class: Class,
    pub method: U12,
}
impl Type {
    pub fn as_u16(&self) -> u16 {
        let class = self.class as u16;
        let method = self.method.as_u12().as_u16();
        ((method & 0b0000_0000_1111) << 0)
            | ((class & 0b01) << 4)
            | ((method & 0b0000_0111_0000) << 5)
            | ((class & 0b10) << 7)
            | ((method & 0b1111_1000_0000) << 9)
    }

    pub fn from_u16(value: u16) -> Result<Self> {
        track_assert!(
            value >> 14 == 0,
            ErrorKind::Invalid,
            "First two-bits of STUN message must be 0"
        );
        let class = ((value >> 4) & 0b01) | ((value >> 7) & 0b10);
        let class = Class::from_u8(class as u8).unwrap();
        let method = (value & 0b0000_0000_1111)
            | ((value >> 1) & 0b0000_0111_0000)
            | ((value >> 2) & 0b1111_1000_0000);
        let method = U12::from_u16(method).unwrap();
        Ok(Type {
            class: class,
            method: method,
        })
    }
}
