use std::io::{self, Write, Read};
use std::net::SocketAddr;
use rand;
use fibers::net::UdpSocket;
use futures::{Future, BoxFuture};
use byteorder::BigEndian;
use byteorder::{WriteBytesExt, ReadBytesExt};

use MAGIC_COOKIE;
use {MessageType, MessageClass, Method};

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
                let buf = vec![0; 1024]; // TODO
                socket.recv_from(buf).map_err(|(_, _, e)| e)
            })
            .and_then(|(_, mut buf, size, _)| {
                buf.truncate(size);
                Message::read_from(&mut &buf[..])
            })
            .boxed()
    }
}

// pub type Binding = BoxFuture<(UdpSocket, Vec<u8>, usize, SocketAddr), io::Error>;
pub type Binding = BoxFuture<Message, io::Error>;

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
        let _message_len = reader.read_u16::<BigEndian>()? as usize;
        let magic_cookie = reader.read_u32::<BigEndian>()?;
        assert_eq!(magic_cookie, MAGIC_COOKIE);
        let mut transaction_id = [0; 12];
        reader.read_exact(&mut transaction_id[..])?;

        // TODO: read attributes
        Ok(Message {
            message_type: message_type,
            transaction_id: transaction_id,
            attributes: Vec::new(),
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

pub type Attribute = u8;
