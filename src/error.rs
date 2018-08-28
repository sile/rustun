use bytecodec;
use fibers::sync::oneshot::MonitorError;
use std;
use std::io;
use std::sync::mpsc::RecvError;
use trackable::error::{self, ErrorKindExt, TrackableError};

use rfc5389::attributes::ErrorCode;
use rfc5389::errors;

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
impl From<bytecodec::Error> for Error {
    fn from(f: bytecodec::Error) -> Self {
        let bytecodec_error_kind = *f.kind();
        let kind = match bytecodec_error_kind {
            bytecodec::ErrorKind::InvalidInput => ErrorKind::Invalid,
            _ => ErrorKind::Other,
        };
        track!(kind.takes_over(f); bytecodec_error_kind).into()
    }
}

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
    Invalid,

    /// The input is valid, but requires unsupported features by this agent.
    Unsupported,

    /// An error specified by the `ErrorCode` instance.
    ErrorCode(ErrorCode),

    /// Other errors.
    Other,
}
impl error::ErrorKind for ErrorKind {
    fn description(&self) -> &str {
        match *self {
            ErrorKind::Timeout => "The operation timed out",
            ErrorKind::Full => "The target resource is full (maybe temporary)",
            ErrorKind::NotStun(_) => "The input bytes are not a STUN message",
            ErrorKind::Invalid => "The input is invalid",
            ErrorKind::Unsupported => {
                "The input is valid, but requires unsupported features by this agent."
            }
            ErrorKind::ErrorCode(ref e) => e.reason_phrase(),
            ErrorKind::Other => "Some error happened",
        }
    }
}
impl From<ErrorKind> for ErrorCode {
    fn from(f: ErrorKind) -> Self {
        match f {
            ErrorKind::Timeout => ErrorCode::new(408, "Request Timeout".to_string()).unwrap(),
            ErrorKind::Full => ErrorCode::new(503, "Service Unavailable".to_string()).unwrap(),
            ErrorKind::NotStun(_) => errors::BadRequest.into(),
            ErrorKind::Invalid => errors::BadRequest.into(),
            ErrorKind::Unsupported => ErrorCode::new(501, "Not Implemented".to_string()).unwrap(),
            ErrorKind::ErrorCode(code) => code,
            ErrorKind::Other => errors::ServerError.into(),
        }
    }
}
