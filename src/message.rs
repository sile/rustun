use std::mem;
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use rand;
use byteorder::{ByteOrder, BigEndian};
use futures::{Future, Async, Poll};
use handy_async::io::{AsyncRead, AsyncWrite};
use handy_async::io::futures::{ReadExact, WriteAll};
use failure::Failure;

use MAGIC_COOKIE;
use {StunMethod, Result, Error};
use types::U12;
use attribute::Attribute;

macro_rules! async_chain_err {
    ($e:expr, $m:expr) => {
        $e.map_err(|e| {
            let (stream, error) = e.map_state(|s| s.0).unwrap();
            (stream, may_fail!(Err(Error::from(error)) as Result<()>).unwrap_err())
        })
    }
}

#[derive(Debug)]
pub struct WriteMessage<W, M, A> {
    future: ::std::result::Result<WriteAll<W, Vec<u8>>, Option<Error>>,
    _phatom: PhantomData<(M, A)>,
}
impl<W, M, A> WriteMessage<W, M, A> {
    fn error(error: Error) -> Self {
        WriteMessage {
            future: Err(Some(error)),
            _phatom: PhantomData,
        }
    }
    fn future(future: WriteAll<W, Vec<u8>>) -> Self {
        WriteMessage {
            future: Ok(future),
            _phatom: PhantomData,
        }
    }
}
impl<W, M, A> Future for WriteMessage<W, M, A>
    where W: Write,
          M: StunMethod,
          A: Attribute
{
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        may_fail!(self.future
            .as_mut()
            .map_err(|e| e.take().unwrap())?
            .poll()
            .map(|ready| ready.map(|(w, _)| w))
            .map_err(|e| Failure::new(e.into_error()).into()))
    }
}

#[derive(Debug)]
pub struct ReadMessage<R, M, A>(ReadMessageInner<R, M, A>);
impl<R, M, A> Future for ReadMessage<R, M, A>
    where R: Read,
          M: StunMethod,
          A: Attribute
{
    type Item = (R, Message<M, A>);
    type Error = (R, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

#[derive(Debug)]
enum ReadMessageInner<R, M, A> {
    Header(ReadExact<R, [u8; 20]>),
    Attrs(Message<M, A>, ReadExact<R, Vec<u8>>),
    Polled,
}
impl<R, M, A> Future for ReadMessageInner<R, M, A>
    where R: Read,
          M: StunMethod,
          A: Attribute
{
    type Item = (R, Message<M, A>);
    type Error = (R, Error);
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, ReadMessageInner::Polled) {
            ReadMessageInner::Header(mut f) => {
                let ok = async_chain_err!(f.poll(),
                                          "Cannot read the header part of a STUN message")?;
                if let Async::Ready((reader, bytes)) = ok {
                    // type
                    let message_type = BigEndian::read_u16(&bytes[0..2]);
                    let message_type = match Type::from_u16(message_type) {
                        Err(e) => return Err((reader, e)),
                        Ok(t) => t,
                    };

                    // length
                    let message_len = BigEndian::read_u16(&bytes[2..4]);

                    // cookie
                    let magic_cookie = BigEndian::read_u32(&bytes[4..8]);
                    if magic_cookie != MAGIC_COOKIE {
                        return Err((reader, Error::UnexpectedMagicCookie(magic_cookie)));
                    }

                    // transaction id
                    let mut transaction_id: [u8; 12] = [0; 12];
                    transaction_id.copy_from_slice(&bytes[8..]);

                    let message = Message {
                        message_type: message_type,
                        transaction_id: transaction_id,
                        attributes: Vec::new(),
                    };

                    let f = reader.async_read_exact(vec![0; message_len as usize]);
                    *self = ReadMessageInner::Attrs(message, f);
                    self.poll()
                } else {
                    *self = ReadMessageInner::Header(f);
                    Ok(Async::NotReady)
                }
            }
            ReadMessageInner::Attrs(mut message, mut f) => {
                let ok = async_chain_err!(f.poll(), "Cannot read the body part of a STUN message")?;
                if let Async::Ready((reader, bytes)) = ok {
                    let bytes_len = bytes.len();
                    let mut attrs_reader = bytes.take(bytes_len as u64);
                    while attrs_reader.limit() > 0 {
                        match A::read_from(&mut attrs_reader) {
                            Err(e) => return Err((reader, e)),
                            Ok(a) => message.add_attribute(a),
                        }
                    }
                    Ok(Async::Ready((reader, message)))
                } else {
                    *self = ReadMessageInner::Attrs(message, f);
                    Ok(Async::NotReady)
                }
            }
            _ => panic!("Cannot poll ReadMessageInner twice"),
        }
    }
}

pub type TransactionId = [u8; 12];

// pub type RawMessage = Message<U12, RawAttribute>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<M, A> {
    message_type: Type<M>,
    transaction_id: [u8; 12],
    attributes: Vec<A>,
}
impl<M, A> Message<M, A>
    where M: StunMethod,
          A: Attribute
{
    pub fn new(message_type: Type<M>) -> Self {
        Message {
            message_type: message_type,
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }
    pub fn class(&self) -> Class {
        self.message_type.class
    }
    pub fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }

    pub fn request(method: M) -> Self {
        Self::new(Type {
            class: Class::Request,
            method: method,
        })
    }
    pub fn indication(method: M) -> Self {
        Self::new(Type {
            class: Class::Indication,
            method: method,
        })
    }
    pub fn success_response(self) -> Self {
        Message {
            message_type: Type {
                class: Class::SuccessResponse,
                method: self.message_type.method,
            },
            transaction_id: self.transaction_id,
            attributes: Vec::new(),
        }
    }
    pub fn failure_response(self) -> Self {
        Message {
            message_type: Type {
                class: Class::ErrorResponse,
                method: self.message_type.method,
            },
            transaction_id: self.transaction_id,
            attributes: Vec::new(),
        }
    }

    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) {
        self.attributes.push(attribute.into());
    }
    pub fn try_from_bytes(mut bytes: &[u8]) -> Result<Self> {
        Self::read_from(&mut bytes).wait().map(|(_, m)| m).map_err(|(_, e)| e)
    }
    pub fn try_into_bytes(self) -> Result<Vec<u8>> {
        self.write_into(Vec::new()).wait()
    }
    pub fn read_from<R: Read>(reader: R) -> ReadMessage<R, M, A> {
        ReadMessage(ReadMessageInner::Header(reader.async_read_exact([0; 20])))
    }
    pub fn write_into<W: Write>(self, writer: W) -> WriteMessage<W, M, A> {
        let mut buf = vec![0; 20];
        BigEndian::write_u16(&mut buf[0..2], self.message_type.as_u16());
        BigEndian::write_u32(&mut buf[4..8], MAGIC_COOKIE);
        buf[8..20].copy_from_slice(&self.transaction_id);

        let mut attr_writer = io::Cursor::new(buf);
        attr_writer.set_position(20);
        for attr in self.attributes.iter() {
            if let Err(e) = attr.write_to(&mut attr_writer) {
                return WriteMessage::error(e);
            }
        }
        let mut buf = attr_writer.into_inner();
        assert!(buf.len() % 4 == 0);

        let attrs_len = buf.len() - 20;
        BigEndian::write_u16(&mut buf[2..4], attrs_len as u16);

        WriteMessage::future(writer.async_write_all(buf))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type<M> {
    pub class: Class,
    pub method: M,
}
impl<M: StunMethod> Type<M> {
    /// TODO:
    ///
    /// ```text
    /// 0                 1
    /// 2  3  4 5 6 7 8 9 0 1 2 3 4 5
    ///
    /// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |M |M |M|M|M|C|M|M|M|C|M|M|M|M|
    /// |11|10|9|8|7|1|6|5|4|0|3|2|1|0|
    /// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
    ///
    /// Figure 3: Format of STUN Message Type Field
    /// ```
    pub fn as_u16(&self) -> u16 {
        let class = self.class as u16;
        let method = self.method.as_u12().as_u16();
        ((method & 0b0000_0000_1111) << 0) | ((class & 0b01) << 4) |
        ((method & 0b0000_0111_0000) << 5) | ((class & 0b10) << 7) |
        ((method & 0b1111_1000_0000) << 9)
    }

    pub fn from_u16(value: u16) -> Result<Self> {
        fail_if!(value >> 14 != 0,
                 Error::NotStunMessage("First two-bits of STUN message must be 0".to_string()))?;
        let class = ((value >> 4) & 0b01) | ((value >> 7) & 0b10);
        let class = Class::from_u8(class as u8).unwrap();

        let method = (value & 0b0000_0000_1111) | ((value >> 1) & 0b0000_0111_0000) |
                     ((value >> 2) & 0b1111_1000_0000);
        let method = U12::from_u16(method).unwrap();
        let method = M::from_u12(method).ok_or(Error::UnknownMethod(method))?;
        Ok(Type {
            class: class,
            method: method,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Class {
    Request = 0b00,
    SuccessResponse = 0b01,
    ErrorResponse = 0b10,
    Indication = 0b11,
}
impl Class {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Class::Request),
            0b01 => Some(Class::Indication),
            0b10 => Some(Class::SuccessResponse),
            0b11 => Some(Class::ErrorResponse),
            _ => None,
        }
    }
    pub fn is_request(&self) -> bool {
        Class::Request == *self
    }
    pub fn is_indication(&self) -> bool {
        Class::Indication == *self
    }
    pub fn is_response(&self) -> bool {
        match *self {
            Class::SuccessResponse | Class::ErrorResponse => true,
            _ => false,
        }
    }
    pub fn expect(&self, expected: Class) -> Result<()> {
        fail_if!(*self != expected, Error::UnexpectedClass(*self, expected))?;
        Ok(())
    }
}
