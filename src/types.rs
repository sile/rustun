//! Miscellaneous types.
use bytecodec::bytes::{BytesDecoder, BytesEncoder};
use bytecodec::fixnum::{U16beDecoder, U16beEncoder, U8Decoder, U8Encoder};
use bytecodec::tuple::{TupleDecoder, TupleEncoder};
use bytecodec::{self, Decode, DecodeExt, Encode, EncodeExt};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

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

    /// Returns a decoder of `SocketAddrValue`.
    pub fn decoder() -> impl Decode<Item = Self> {
        let base: TupleDecoder<(U8Decoder, U8Decoder, U16beDecoder)> = Default::default();
        base.try_map(|(_, family, port)| -> bytecodec::Result<_> {
            track_assert!(
                family == 1 || family == 2,
                bytecodec::ErrorKind::InvalidInput,
                "Unsupported address family: {}",
                family
            );
            Ok((family, port))
        }).and_then(|(family, port)| {
            let ip = match family {
                1 => BytesDecoder::new(IpBytes::V4([0; 4])),
                2 => BytesDecoder::new(IpBytes::V6([0; 16])),
                _ => unreachable!(),
            };
            ip.map(move |ip| {
                let ip = match ip {
                    IpBytes::V4(bytes) => IpAddr::from(Ipv4Addr::from(bytes)),
                    IpBytes::V6(bytes) => IpAddr::from(Ipv6Addr::from(bytes)),
                };
                SocketAddrValue(SocketAddr::new(ip, port))
            })
        })
    }

    /// Returns an encoder of `SocketAddrValue`.
    pub fn encoder() -> impl Encode<Item = Self> {
        let base: TupleEncoder<(U8Encoder, U8Encoder, U16beEncoder, BytesEncoder<IpBytes>)> =
            Default::default();
        base.map_from(|SocketAddrValue(addr)| {
            let kind = if addr.ip().is_ipv4() { 1 } else { 2 };
            (0, kind, addr.port(), IpBytes::new(addr.ip()))
        })
    }
}

/// An attempted cheap reference-to-reference conversion.
pub trait TryAsRef<T> {
    /// Performs the conversion.
    fn try_as_ref(&self) -> Option<&T>;
}

enum IpBytes {
    V4([u8; 4]),
    V6([u8; 16]),
}
impl IpBytes {
    fn new(ip: IpAddr) -> Self {
        match ip {
            IpAddr::V4(ip) => IpBytes::V4(ip.octets()),
            IpAddr::V6(ip) => IpBytes::V6(ip.octets()),
        }
    }
}
impl AsRef<[u8]> for IpBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            IpBytes::V4(bytes) => bytes,
            IpBytes::V6(bytes) => bytes,
        }
    }
}
impl AsMut<[u8]> for IpBytes {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            IpBytes::V4(bytes) => bytes,
            IpBytes::V6(bytes) => bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_addr_value_encoder_works() {
        let mut encoder = SocketAddrValue::encoder();

        let v4addr = "127.0.0.1:80".parse().unwrap();
        let bytes = encoder
            .encode_into_bytes(SocketAddrValue::new(v4addr))
            .unwrap();
        assert_eq!(bytes, [0, 1, 0, 80, 127, 0, 0, 1]);

        let v6addr = "[::]:90".parse().unwrap();
        let bytes = encoder
            .encode_into_bytes(SocketAddrValue::new(v6addr))
            .unwrap();
        assert_eq!(
            bytes,
            [0, 2, 0, 90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn socket_addr_value_decoder_works() {
        let mut decoder = SocketAddrValue::decoder();

        let v4addr = decoder
            .decode_from_bytes(&[0, 1, 0, 80, 127, 0, 0, 1])
            .unwrap();
        assert_eq!(v4addr.0.to_string(), "127.0.0.1:80");

        let v6addr = decoder
            .decode_from_bytes(&[0, 2, 0, 90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            .unwrap();
        assert_eq!(v6addr.0.to_string(), "[::]:90");
    }
}
