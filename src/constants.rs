//! Constants related to STUN.

/// The magic cookie value.
///
/// > The magic cookie field **MUST** contain the fixed value `0x2112A442` in
/// > network byte order.
/// > In [RFC 3489](https://tools.ietf.org/html/rfc3489), this field was part of
/// > the transaction ID; placing the magic cookie in this location allows
/// > a server to detect if the client will understand certain attributes
/// > that were added in this revised specification.  In addition, it aids
/// > in distinguishing STUN packets from packets of other protocols when
/// > STUN is multiplexed with those other protocols on the same port.
/// >
/// > ([RFC 5389 -- 6. STUN Message Structure](https://tools.ietf.org/html/rfc5389#section-6))
pub const MAGIC_COOKIE: u32 = 0x2112A442;

pub const DEFAULT_PORT: u16 = 3478;
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 548;
pub const DEFAULT_RTO_MS: u64 = 500;
pub const DEFAULT_RTO_CACHE_DURATION_MS: u64 = 10 * 60 * 1000;
pub const DEFAULT_RC: u32 = 7;
pub const DEFAULT_RM: u32 = 16;
pub const DEFAULT_TI_MS: u64 = 39_500;
pub const DEFAULT_MAX_CLIENT_CONCURRENCY: usize = 10;
pub const DEFAULT_MIN_TRANSACTION_INTERVAL_MS: u64 = DEFAULT_RTO_MS;
