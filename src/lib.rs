extern crate rand;
extern crate fibers;
extern crate futures;
extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate handy_async;

pub use transport::Transport;
pub use attribute::Attribute;

use types::U12;

pub mod client;
pub mod server;
pub mod message;
pub mod attribute;
pub mod types;
pub mod rfc5389;
pub mod transport;

pub const DEFAULT_PORT: u16 = 3478;
pub const DEFAULT_TLS_PORT: u16 = 5349;

pub const MAGIC_COOKIE: u32 = 0x2112A442;

pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 568;

// TODO: rename
pub trait StunMethod: Sized {
    fn from_u12(value: U12) -> Option<Self>;
    fn as_u12(&self) -> U12;
}

error_chain!{
    errors {
        UnknownMethod(method: U12) {
        }
        UnknownAttribute(attr_type: u16) {
        }
        UnexpectedMagicCookie(cookie: u32) {
        }
        UnexpectedClass(actual: message::Class, expected: message::Class) {
        }
        NotResponse(class: message::Class) {
        }
        ChannelDisconnected
        ChannelFull
    }
    foreign_links {
        Io(std::io::Error);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
