use std::net::SocketAddr;
use fibers::Spawn;
use fibers::net::UdpSocket;
use trackable::error::ErrorKindExt;

use {Result, Client, ErrorKind};
use message::{self, Class, RawMessage};
use attribute::{self, RawAttribute};
use types::U12;
use clients;

pub mod methods;
pub mod attributes;

type UdpClientInner = clients::RateLimitedClient<clients::UdpClient, Method, Attribute>;

#[derive(Debug)]
pub struct UdpClient(UdpClientInner);
impl UdpClient {
    pub fn new<T: Spawn>(spawner: T, socket: UdpSocket, server: SocketAddr) -> Self {
        let inner = clients::UdpClient::new(spawner, socket, server);
        UdpClient(clients::RateLimitedClient::new(inner))
    }
    pub fn inner(&self) -> &UdpClientInner {
        &self.0
    }
    pub fn inner_mut(&mut self) -> &mut UdpClientInner {
        &mut self.0
    }
}
impl Client<Method, Attribute> for UdpClient {
    type Call = <UdpClientInner as Client<Method, Attribute>>::Call;
    type Cast = <UdpClientInner as Client<Method, Attribute>>::Cast;
    fn call(&mut self, message: Request) -> Self::Call {
        self.0.call(message)
    }
    fn cast(&mut self, message: Indication) -> Self::Cast {
        self.0.cast(message)
    }
}

pub type TcpClient = clients::TcpClient;

pub type Message = ::Message<Method, Attribute>;
pub type Request = message::Request<Method, Attribute>;
pub type Response = message::Response<Method, Attribute>;
pub type Indication = message::Indication<Method, Attribute>;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Method {
    Binding(methods::Binding),
}
impl Method {
    pub fn binding() -> Self {
        Method::Binding(methods::Binding)
    }
}
impl ::Method for Method {
    fn from_u12(value: U12) -> Option<Self> {
        match value.as_u16() {
            methods::METHOD_BINDING => Some(Method::Binding(methods::Binding)),
            _ => None,
        }
    }
    fn as_u12(&self) -> U12 {
        match *self {
            Method::Binding(ref m) => m.as_u12(),
        }
    }
    fn permits_class(&self, class: Class) -> bool {
        match *self {
            Method::Binding(ref m) => m.permits_class(class),
        }
    }
}

macro_rules! impl_attr_from {
    ($attr:ident) => {
        impl From<attributes::$attr> for Attribute {
            fn from(f: attributes::$attr) -> Self {
                Attribute::$attr(f)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Attribute {
    XorMappedAddress(attributes::XorMappedAddress),
}
impl_attr_from!(XorMappedAddress);
impl ::Attribute for Attribute {
    fn get_type(&self) -> attribute::Type {
        match *self {
            Attribute::XorMappedAddress(ref a) => a.get_type(),
        }
    }
    fn decode(attr: &RawAttribute, message: &RawMessage) -> Result<Self> {
        match attr.get_type().as_u16() {
            attributes::TYPE_XOR_MAPPED_ADDRESS => {
                attributes::XorMappedAddress::decode(attr, message).map(From::from)
            }
            t => Err(ErrorKind::Unsupported.cause(format!("Unknown attribute: type={}", t))),
        }
    }
    fn encode_value(&self, message: &RawMessage) -> Result<Vec<u8>> {
        match *self {
            Attribute::XorMappedAddress(ref a) => a.encode_value(message),
        }
    }
}
