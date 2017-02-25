//! Miscellaneous types.

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

/// TODO: move to rfc5389
#[derive(Debug, Clone)]
pub struct ErrorCode {
    code: u16,
    reason_phrase: String,
}
impl ErrorCode {
    pub fn new(code: u16, reason_phrase: &str) -> Option<Self> {
        if 300 <= code && code < 600 {
            Some(ErrorCode {
                code: code,
                reason_phrase: reason_phrase.to_string(),
            })
        } else {
            None
        }
    }
    pub fn code(&self) -> u16 {
        self.code
    }
    pub fn reason_phrase(&self) -> &str {
        &self.reason_phrase
    }
}
