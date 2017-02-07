use std::io::{self, Write, Read};
use std::net::SocketAddr;
use rand;
use fibers::net::UdpSocket;
use futures::{Future, BoxFuture};
use byteorder::BigEndian;
use byteorder::{WriteBytesExt, ReadBytesExt};

use {MAGIC_COOKIE, DEFAULT_MAX_MESSAGE_SIZE};
use {Result, AttrType, Error};
use message::{Type, Class};

// TODO:
use rfc5389::Method;

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
            .map_err(Error::from)
            .and_then(|(_, mut buf, size, _)| {
                buf.truncate(size);
                Message::read_from(&mut &buf[..])
            })
            .boxed()
    }
}

// pub type Binding = BoxFuture<(UdpSocket, Vec<u8>, usize, SocketAddr), io::Error>;
pub type Binding = BoxFuture<Message, Error>;

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
        let mut reader = reader.take(length as u64);
        Ok(match AttrType::from_u16(attr_type) {
            AttrType::XorMappedAddress => {
                Attribute::XorMappedAddress(XorMappedAddress::read_from(&mut reader)?)
            }
            other => Attribute::Unknown(UnknownAttr::read_from(&mut reader, other.as_u16())?),
        })
    }
}

#[derive(Debug)]
pub struct Message {
    message_type: Type<::rfc5389::Method>,
    //message_len: u16,
    //magic_cookie: u32,
    transaction_id: [u8; 12],
    attributes: Vec<Attribute>,
}
impl Message {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let message_type = reader.read_u16::<BigEndian>()?;
        assert_eq!(message_type >> 14, 0);
        let message_type = Type::from_u16(message_type)?;
        let message_len = reader.read_u16::<BigEndian>()? as usize;
        let magic_cookie = reader.read_u32::<BigEndian>()?;
        assert_eq!(magic_cookie, MAGIC_COOKIE);
        let mut transaction_id = [0; 12];
        reader.read_exact(&mut transaction_id[..])?;

        let mut reader = reader.take(message_len as u64);
        let mut attrs = Vec::new();
        while reader.limit() > 0 {
            attrs.push(Attribute::read_from(&mut reader)?);
        }

        Ok(Message {
            message_type: message_type,
            transaction_id: transaction_id,
            attributes: attrs,
        })
    }
    pub fn new(message_type: Type<::rfc5389::Method>) -> Self {
        Message {
            message_type: message_type,
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }
    pub fn binding_request() -> Self {
        Self::new(Type {
            class: Class::Request,
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
