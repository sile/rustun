//! Miscellaneous types.
use std::io::{Read, Write};
use std::net::{SocketAddr, IpAddr};
use handy_async::sync_io::{ReadExt, WriteExt};
use trackable::error::ErrorKindExt;

use {Result, ErrorKind};
use constants;

/// Unsigned 12 bit integer.
#[derive(Debug, Default, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct U12(u16);
impl U12 {
    /// Converts from `u8` value.
    pub fn from_u8(value: u8) -> Self {
        U12(value as u16)
    }

    /// Tries to convert from `u16` value.
    ///
    /// If `value` is greater than `0xFFF`, this will return `None`.
    pub fn from_u16(value: u16) -> Option<Self> {
        if value < 0x1000 {
            Some(U12(value))
        } else {
            None
        }
    }

    /// Converts to `u16` value.
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

/// Transaction ID.
///
/// > STUN is a client-server protocol.  It supports two types of
/// > transactions.  One is a request/response transaction in which a
/// > client sends a request to a server, and the server returns a
/// > response.  The second is an indication transaction in which either
/// > agent -- client or server -- sends an indication that generates no
/// > response.  Both types of transactions include a **transaction ID**, which
/// > is a randomly selected 96-bit number.  For request/response
/// > transactions, this transaction ID allows the client to associate the
/// > response with the request that generated it; for indications, the
/// > transaction ID serves as a debugging aid.
/// >
/// > ([RFC 5389 -- 3. Overview of Operation](https://tools.ietf.org/html/rfc5389#section-3))
pub type TransactionId = [u8; 12];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Socket address used as STUN attribute value.
pub struct SocketAddrValue(SocketAddr);
impl SocketAddrValue {
    /// Makes a new `SocketAddrValue` instance.
    pub fn new(addr: SocketAddr) -> Self {
        SocketAddrValue(addr)
    }

    /// Returns the socket address of this instance.
    pub fn address(&self) -> SocketAddr {
        self.0
    }

    /// Applies XOR operation on the socket address of this instance.
    pub fn xor(&self, transaction_id: &TransactionId) -> Self {
        let addr = self.0;
        let xor_port = addr.port() ^ (constants::MAGIC_COOKIE >> 16) as u16;
        let xor_addr = match addr.ip() {
            IpAddr::V4(ip) => {
                let mut octets = ip.octets();
                for i in 0..octets.len() {
                    octets[i] ^= (constants::MAGIC_COOKIE >> (24 - i * 8)) as u8;
                }
                let xor_ip = From::from(octets);
                SocketAddr::new(IpAddr::V4(xor_ip), xor_port)
            }
            IpAddr::V6(ip) => {
                let mut octets = ip.octets();
                for i in 0..4 {
                    octets[i] ^= (constants::MAGIC_COOKIE >> (24 - i * 8)) as u8;
                }
                for i in 4..16 {
                    octets[i] ^= transaction_id[i - 4];
                }
                let xor_ip = From::from(octets);
                SocketAddr::new(IpAddr::V6(xor_ip), xor_port)
            }
        };
        Self::new(xor_addr)
    }

    /// Reads a `SocketAddrValue` instance from `reader`.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let _ = track_try!(reader.read_u8());
        let family = track_try!(reader.read_u8());
        let port = track_try!(reader.read_u16be());
        let ip = match family {
            1 => {
                let ip = track_try!(reader.read_u32be());
                IpAddr::V4(From::from(ip))
            }
            2 => {
                let mut octets = [0; 16];
                track_try!(reader.read_exact(&mut octets[..]));
                IpAddr::V6(From::from(octets))
            }
            _ => {
                let message = format!("Unsupported address family: {}", family);
                return Err(ErrorKind::Unsupported.cause(message).into());
            }
        };
        Ok(Self::new(SocketAddr::new(ip, port)))
    }

    /// Writes the socket address of this instance to `writer`.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let addr = self.0;
        track_try!(writer.write_u8(0));
        match addr.ip() {
            IpAddr::V4(ip) => {
                track_try!(writer.write_u8(1));
                track_try!(writer.write_u16be(addr.port()));
                track_try!(writer.write_all(&ip.octets()));
            }
            IpAddr::V6(ip) => {
                track_try!(writer.write_u8(2));
                track_try!(writer.write_u16be(addr.port()));
                track_try!(writer.write_all(&ip.octets()));
            }
        }
        Ok(())
    }
}


/// An attempted cheap reference-to-reference conversion.
pub trait TryAsRef<T> {
    /// Performs the conversion.
    fn try_as_ref(&self) -> Option<&T>;
}
