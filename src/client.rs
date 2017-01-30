use std::io::{self, Write, Read};
use std::net::SocketAddr;
use rand;
use fibers::net::UdpSocket;
use futures::{Future, BoxFuture};
use byteorder::BigEndian;
use byteorder::{WriteBytesExt, ReadBytesExt};

use {MAGIC_COOKIE, DEFAULT_MAX_MESSAGE_SIZE};
use {MessageType, MessageClass, Method};
use AttrType;

pub struct StunClient {}
impl StunClient {
    pub fn new() -> Self {
        StunClient {}
    }
    pub fn binding(self, addr: SocketAddr) -> Binding {
        let local_addr = "0.0.0.0:0".parse().unwrap();
        UdpSocket::bind(local_addr)
            .and_then(move |socket| {
                let m = Message::binding_request();
                socket.send_to(m.into_bytes(), addr).map_err(|(_, _, e)| e)
            })
            .and_then(|(socket, _, _)| {
                let buf = vec![0; DEFAULT_MAX_MESSAGE_SIZE];
                socket.recv_from(buf).map_err(|(_, _, e)| e)
            })
            .and_then(|(_, mut buf, size, _)| {
                buf.truncate(size);
                Message::read_from(&mut &buf[..])
            })
            .boxed()
    }
}

pub struct FixedLengthReader<R> {
    inner: R,
    length: usize,
}
impl<R: Read> FixedLengthReader<R> {
    pub fn new(inner: R, length: usize) -> Self {
        FixedLengthReader {
            inner: inner,
            length: length,
        }
    }
    pub fn is_eos(&self) -> bool {
        self.length == 0
    }
    pub fn into_inner(self) -> R {
        self.inner
    }
}
impl<R: Read> Read for FixedLengthReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use std::cmp;
        if self.length == 0 {
            Ok(0)
        } else {
            let size = cmp::min(self.length, buf.len());
            let size = self.inner.read(&mut buf[0..size])?;
            self.length -= size;
            Ok(size)
        }
    }
}

// pub type Binding = BoxFuture<(UdpSocket, Vec<u8>, usize, SocketAddr), io::Error>;
pub type Binding = BoxFuture<Message, io::Error>;

#[derive(Debug)]
pub struct XorMappedAddress {
    pub address: SocketAddr,
}
impl XorMappedAddress {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
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
}
#[derive(Debug)]
pub struct UnknownAttr {
    pub attr_type: u16,
    pub value: Vec<u8>,
}
impl UnknownAttr {
    // TODO: 'error-chain'を試す
    pub fn read_from<R: Read>(reader: &mut R, attr_type: u16) -> io::Result<Self> {
        let mut value = Vec::new();
        reader.read_to_end(&mut value)?;
        Ok(UnknownAttr {
            attr_type: attr_type,
            value: value,
        })
    }
}

#[derive(Debug)]
pub enum Attribute {
    XorMappedAddress(XorMappedAddress),
    Unknown(UnknownAttr),
}
impl Attribute {
    // TODO: 'error-chain'を試す
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let attr_type = reader.read_u16::<BigEndian>()?;
        let length = reader.read_u16::<BigEndian>()?;
        let reader = &mut FixedLengthReader::new(reader, length as usize);
        Ok(match AttrType::from_u16(attr_type) {
            AttrType::XorMappedAddress => {
                Attribute::XorMappedAddress(XorMappedAddress::read_from(reader)?)
            }
            other => Attribute::Unknown(UnknownAttr::read_from(reader, other.as_u16())?),
        })
    }
}

#[derive(Debug)]
pub struct Message {
    message_type: MessageType,
    //message_len: u16,
    //magic_cookie: u32,
    transaction_id: [u8; 12],
    attributes: Vec<Attribute>,
}
impl Message {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let message_type = reader.read_u16::<BigEndian>()?;
        assert_eq!(message_type >> 14, 0);
        let message_type = MessageType::from_u16(message_type);
        let message_len = reader.read_u16::<BigEndian>()? as usize;
        let magic_cookie = reader.read_u32::<BigEndian>()?;
        assert_eq!(magic_cookie, MAGIC_COOKIE);
        let mut transaction_id = [0; 12];
        reader.read_exact(&mut transaction_id[..])?;

        let mut reader = FixedLengthReader::new(reader, message_len);
        let mut attrs = Vec::new();
        while !reader.is_eos() {
            attrs.push(Attribute::read_from(&mut reader)?);
        }

        Ok(Message {
            message_type: message_type,
            transaction_id: transaction_id,
            attributes: attrs,
        })
    }
    pub fn new(message_type: MessageType) -> Self {
        Message {
            message_type: message_type,
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }
    pub fn binding_request() -> Self {
        Self::new(MessageType {
            class: MessageClass::Request,
            method: Method::Binding,
        })
    }
    pub fn into_bytes(self) -> Vec<u8> {
        let buf = Vec::new();
        let mut writer = io::Cursor::new(buf);
        writer.write_u16::<BigEndian>(self.message_type.as_u16()).unwrap();
        writer.write_u16::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(MAGIC_COOKIE).unwrap();
        writer.write_all(&self.transaction_id).unwrap();

        assert!(self.attributes.is_empty());

        let mut buf = writer.into_inner();
        // TODO: padding
        assert!(buf.len() % 4 == 0);

        let body_len = buf.len() - 20;
        (&mut buf[2..4]).write_u16::<BigEndian>(body_len as u16).unwrap();
        buf
    }
}
