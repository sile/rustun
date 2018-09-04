use fibers::sync::oneshot::MonitorError;
use std;
use std::io;
use std::sync::mpsc::RecvError;
use stun_codec::rfc5389::attributes::ErrorCode;
use stun_codec::rfc5389::errors;
use stun_codec::AttributeType;
use trackable::error::{self, ErrorKindExt, TrackableError};

/// The error type for this crate.
#[derive(Debug, Clone)]
pub struct Error(TrackableError<ErrorKind>);
derive_traits_for_trackable_error_newtype!(Error, ErrorKind);
impl From<MonitorError<Error>> for Error {
    fn from(f: MonitorError<Error>) -> Self {
        f.unwrap_or(ErrorKind::Other.into())
    }
}
impl From<io::Error> for Error {
    fn from(f: io::Error) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}
impl From<RecvError> for Error {
    fn from(f: RecvError) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}
impl From<std::time::SystemTimeError> for Error {
    fn from(f: std::time::SystemTimeError) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}

// TODO:
/// A list of error kind.
#[derive(Debug, Clone)]
pub enum ErrorKind {
    /// The operation timed out.
    Timeout,

    /// The target resource is full (maybe temporary).
    Full,

    /// The input bytes are not a STUN message.
    NotStun(Vec<u8>),

    /// The input is invalid.
    InvalidInput,

    /// The input is valid, but requires unsupported features by this agent.
    Unsupported,

    /// TODO
    UnknownAttributes(Vec<AttributeType>),

    /// An error specified by the `ErrorCode` instance.
    ErrorCode(ErrorCode),

    /// Other errors.
    Other,
}
impl error::ErrorKind for ErrorKind {}
impl From<ErrorKind> for ErrorCode {
    fn from(f: ErrorKind) -> Self {
        match f {
            ErrorKind::Timeout => ErrorCode::new(408, "Request Timeout".to_string()).unwrap(),
            ErrorKind::Full => ErrorCode::new(503, "Service Unavailable".to_string()).unwrap(),
            ErrorKind::NotStun(_) => errors::BadRequest.into(),
            ErrorKind::InvalidInput => errors::BadRequest.into(),
            ErrorKind::Unsupported => ErrorCode::new(501, "Not Implemented".to_string()).unwrap(),
            ErrorKind::UnknownAttributes(_) => errors::UnknownAttribute.into(),
            ErrorKind::ErrorCode(code) => code,
            ErrorKind::Other => errors::ServerError.into(),
        }
    }
}
