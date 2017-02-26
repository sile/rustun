//! Constant values related to STUN.

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

/// The default TCP and UDP port for STUN.
pub const DEFAULT_PORT: u16 = 3478;

/// The default TLS port for STUN.
pub const DEFAULT_TLS_PORT: u16 = 5349;

/// The default maximum size of a message.
///
/// > All STUN messages sent over UDP SHOULD be less than the path MTU, if
/// > known.  If the path MTU is unknown, messages SHOULD be the smaller of
/// > 576 bytes and the first-hop MTU for IPv4 [RFC1122] and 1280 bytes for
/// > IPv6 [RFC2460].  This value corresponds to the overall size of the IP
/// > packet.  Consequently, for IPv4, the actual STUN message would need
/// > to be less than **548 bytes** (576 minus 20-byte IP header, minus 8-byte
/// > UDP header, assuming no IP options are used).
/// >
/// > ([RFC 5389 -- 7.1. Forming a Request or an Indication]
/// > (https://tools.ietf.org/html/rfc5389#section-7.1))
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 548;

/// The default value of RTO (Retransmission TimeOut).
///
/// > A client SHOULD retransmit a STUN request message starting with an
/// > interval of RTO ("Retransmission TimeOut"), doubling after each
/// > retransmission.  The RTO is an estimate of the round-trip time (RTT),
/// > and is computed as described in RFC 2988 [RFC2988], with two
/// > exceptions.  First, the initial value for RTO SHOULD be configurable
/// > (rather than the 3 s recommended in RFC 2988) and SHOULD be greater
/// > than **500 ms**.
/// >
/// > ([RFC 5389 -- 7.2.1. Sending over UDP]
/// > (https://tools.ietf.org/html/rfc5389#section-7.2.1))
pub const DEFAULT_RTO_MS: u64 = 500;

/// The default duration preserving a cached RTO (Retransmission TimeOut).
///
/// > The value for RTO SHOULD be cached by a client after the completion
/// > of the transaction, and used as the starting value for RTO for the
/// > next transaction to the same server (based on equality of IP
/// > address).  The value SHOULD be considered stale and discarded after
/// > **10 minutes**.
/// >
/// > ([RFC 5389 -- 7.2.1. Sending over UDP]
/// > (https://tools.ietf.org/html/rfc5389#section-7.2.1))
pub const DEFAULT_RTO_CACHE_DURATION_MS: u64 = 10 * 60 * 1000;

/// The default value of Ti (Timeout duration for a request over TCP).
///
/// > Reliability of STUN over TCP and TLS-over-TCP is handled by TCP
/// > itself, and there are no retransmissions at the STUN protocol level.
/// > However, for a request/response transaction, if the client has not
/// > received a response by **Ti** seconds after it sent the SYN to establish
/// > the connection, it considers the transaction to have timed out.  **Ti**
/// > SHOULD be configurable and SHOULD have a default of **39.5s**.
/// >
/// > ([RFC 5389 -- 7.2.2. Sending over TCP or TLS-over-TCP]
/// > (https://tools.ietf.org/html/rfc5389#section-7.2.2))
pub const DEFAULT_TIMEOUT_MS: u64 = 39_500;

/// The default max concurrent transactions by a client to a server.
///
/// > At any time, a client MAY have multiple outstanding STUN requests
/// > with the same STUN server (that is, multiple transactions in
/// > progress, with different transaction IDs).  Absent other limits to
/// > the rate of new transactions (such as those specified by ICE for
/// > connectivity checks or when STUN is run over TCP), a client SHOULD
/// > space new transactions to a server by RTO and SHOULD limit itself to
/// > **ten outstanding transactions** to the same server.
/// >
/// > ([RFC 5389 -- 7.2. Sending the Request or Indication]
/// > (https://tools.ietf.org/html/rfc5389#section-7.2))
pub const DEFAULT_MAX_OUTSTANDING_TRANSACTIONS: usize = 10;

/// The default interval between transactions issued by a client to a serve.
///
/// > At any time, a client MAY have multiple outstanding STUN requests
/// > with the same STUN server (that is, multiple transactions in
/// > progress, with different transaction IDs).  Absent other limits to
/// > the rate of new transactions (such as those specified by ICE for
/// > connectivity checks or when STUN is run over TCP), **a client SHOULD
/// > space new transactions to a server by RTO** and SHOULD limit itself to
/// > ten outstanding transactions to the same server.
/// >
/// > ([RFC 5389 -- 7.2. Sending the Request or Indication]
/// > (https://tools.ietf.org/html/rfc5389#section-7.2))
pub const DEFAULT_MIN_TRANSACTION_INTERVAL_MS: u64 = DEFAULT_RTO_MS;
