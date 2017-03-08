//! Error codes that are defined in [RFC 5389 -- 15.6 ERROR-CODE]
//! (https://tools.ietf.org/html/rfc5389#section-15.6).
use rfc5389::attributes::ErrorCode;

/// `300`: "Try Alternate".
///
/// > The client should contact an alternate server for
/// > this request.  This error response MUST only be sent if the
/// > request included a USERNAME attribute and a valid MESSAGE-
/// > INTEGRITY attribute; otherwise, it MUST NOT be sent and error
/// > code 400 (Bad Request) is suggested.  This error response MUST
/// > be protected with the MESSAGE-INTEGRITY attribute, and receivers
/// > MUST validate the MESSAGE-INTEGRITY of this response before
/// > redirecting themselves to an alternate server.
/// >
/// > > Note: Failure to generate and validate message integrity
/// > > for a 300 response allows an on-path attacker to falsify a
/// > > 300 response thus causing subsequent STUN messages to be
/// > > sent to a victim.
/// >
/// > [RFC 5389 -- 15.6 ERROR-CODE](https://tools.ietf.org/html/rfc5389#section-15.6)
#[derive(Debug, Clone, Copy)]
pub struct TryAlternate;
impl From<TryAlternate> for ErrorCode {
    fn from(_: TryAlternate) -> Self {
        ErrorCode::new(300, "Try Alternate".to_string()).unwrap()
    }
}

/// `400`: "Bad Request".
///
/// > The request was malformed.  The client SHOULD NOT
/// > retry the request without modification from the previous
/// > attempt.  The server may not be able to generate a valid
/// > MESSAGE-INTEGRITY for this error, so the client MUST NOT expect
/// > a valid MESSAGE-INTEGRITY attribute on this response.
/// >
/// > [RFC 5389 -- 15.6 ERROR-CODE](https://tools.ietf.org/html/rfc5389#section-15.6)
#[derive(Debug, Clone, Copy)]
pub struct BadRequest;
impl From<BadRequest> for ErrorCode {
    fn from(_: BadRequest) -> Self {
        ErrorCode::new(400, "Bad Request".to_string()).unwrap()
    }
}

/// `401`: "Unauthorized".
///
/// > The request did not contain the correct
/// > credentials to proceed.  The client should retry the request
/// > with proper credentials.
/// >
/// > [RFC 5389 -- 15.6 ERROR-CODE](https://tools.ietf.org/html/rfc5389#section-15.6)
#[derive(Debug, Clone, Copy)]
pub struct Unauthorized;
impl From<Unauthorized> for ErrorCode {
    fn from(_: Unauthorized) -> Self {
        ErrorCode::new(401, "Unauthorized".to_string()).unwrap()
    }
}

/// `420`: "Unknown Attribute".
///
/// > The server received a STUN packet containing
/// > a comprehension-required attribute that it did not understand.
/// > The server MUST put this unknown attribute in the UNKNOWN-
/// > ATTRIBUTE attribute of its error response.
/// >
/// > [RFC 5389 -- 15.6 ERROR-CODE](https://tools.ietf.org/html/rfc5389#section-15.6)
#[derive(Debug, Clone, Copy)]
pub struct UnknownAttribute;
impl From<UnknownAttribute> for ErrorCode {
    fn from(_: UnknownAttribute) -> Self {
        ErrorCode::new(420, "Unknown Attribute".to_string()).unwrap()
    }
}

/// `438`: "Stale Nonce".
///
/// > The NONCE used by the client was no longer valid.
/// > The client should retry, using the NONCE provided in the
/// > response.
/// >
/// > [RFC 5389 -- 15.6 ERROR-CODE](https://tools.ietf.org/html/rfc5389#section-15.6)
#[derive(Debug, Clone, Copy)]
pub struct StaleNonce;
impl From<StaleNonce> for ErrorCode {
    fn from(_: StaleNonce) -> Self {
        ErrorCode::new(438, "Stale Nonce".to_string()).unwrap()
    }
}

/// `500`: "Server Error".
///
/// > The server has suffered a temporary error.  The
/// > client should try again.
/// >
/// > [RFC 5389 -- 15.6 ERROR-CODE](https://tools.ietf.org/html/rfc5389#section-15.6)
#[derive(Debug, Clone, Copy)]
pub struct ServerError;
impl From<ServerError> for ErrorCode {
    fn from(_: ServerError) -> Self {
        ErrorCode::new(500, "Server Error".to_string()).unwrap()
    }
}
