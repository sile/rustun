extern crate rand;
extern crate fibers;
extern crate futures;
extern crate byteorder;
extern crate handy_async;
#[macro_use]
extern crate failure;

pub use transport::Transport;
pub use attribute::{Attribute, AttributeType};

use types::U12;

pub mod client;
pub mod server;
pub mod message;
pub mod attribute;
pub mod attr;
pub mod msg;
pub mod types;
pub mod transport;
pub mod transport2;
pub mod io;
pub mod rfc5389;

pub mod constants {
    pub const DEFAULT_MESSAGE_BUFFER_SIZE: usize = 548;
}

pub const DEFAULT_PORT: u16 = 3478;
pub const DEFAULT_TLS_PORT: u16 = 5349;

pub const MAGIC_COOKIE: u32 = 0x2112A442;

pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 568;

pub trait StunMethod: Sized {
    fn from_u12(value: U12) -> Option<Self>;
    fn as_u12(&self) -> U12;
    fn permits_class(&self, class: message::Class) -> bool;
}
impl StunMethod for U12 {
    fn from_u12(value: U12) -> Option<Self> {
        Some(value)
    }
    fn as_u12(&self) -> U12 {
        *self
    }
    fn permits_class(&self, _class: message::Class) -> bool {
        true
    }
}

pub trait Protocol {
    type Method;
    type Attribute: Attribute;
    type ErrorCode;
}

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NotStunMessage(String),
    UnknownMethod(U12),
    UnknownAttribute(u16),
    UnexpectedMagicCookie(u32),
    UnexpectedClass(message::Class, message::Class),
    NotResponse(message::Class),
    ChannelDisconnected,
    ChannelFull,
    Failed(failure::Failure),
}
impl Error {
    pub fn failed<E>(error: E) -> Self
        where E: Into<Box<std::error::Error + Send + Sync>>
    {
        Error::from(failure::Failure::new(error))
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Failed(ref failure) => write!(f, "{}", failure),
            _ => unimplemented!(),
        }
    }
}
impl std::error::Error for Error {
    fn description(&self) -> &str {
        unimplemented!()
    }
    fn cause(&self) -> Option<&std::error::Error> {
        unimplemented!()
    }
}
impl From<failure::Failure> for Error {
    fn from(f: failure::Failure) -> Self {
        Error::Failed(f)
    }
}
impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        Error::Failed(failure::Failure::new(f))
    }
}
impl failure::MaybeFailure for Error {
    fn try_as_failure_mut(&mut self) -> Option<&mut failure::Failure> {
        if let Error::Failed(ref mut f) = *self {
            Some(f)
        } else {
            None
        }
    }
    fn try_into_failure(self) -> ::std::result::Result<failure::Failure, Self> {
        if let Error::Failed(f) = self {
            Ok(f)
        } else {
            Err(self)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
