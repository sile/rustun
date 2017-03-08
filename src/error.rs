use std;
use std::io;
use std::sync::mpsc::RecvError;
use trackable::error::{self, IntoTrackableError, TrackableError, ErrorKindExt};
use fibers::sync::oneshot::MonitorError;

use rfc5389::attributes::ErrorCode;
use rfc5389::errors;

/// The error type for this crate.
pub type Error = TrackableError<ErrorKind>;

/// A list of error kind.
#[derive(Debug, Clone)]
pub enum ErrorKind {
    /// The operation timed out.
    Timeout,

    /// The target resource is full (maybe temporary).
    Full,

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
            ErrorKind::Invalid => errors::BadRequest.into(),
            ErrorKind::Unsupported => ErrorCode::new(501, "Not Implemented".to_string()).unwrap(),
            ErrorKind::ErrorCode(code) => code,
            ErrorKind::Other => errors::ServerError.into(),
        }
    }
}
impl IntoTrackableError<MonitorError<Error>> for ErrorKind {
    fn into_trackable_error(f: MonitorError<Error>) -> Error {
        f.unwrap_or(ErrorKind::Other.into())
    }
}
impl IntoTrackableError<io::Error> for ErrorKind {
    fn into_trackable_error(f: io::Error) -> Error {
        ErrorKind::Other.cause(f)
    }
}
impl IntoTrackableError<RecvError> for ErrorKind {
    fn into_trackable_error(f: RecvError) -> Error {
        ErrorKind::Other.cause(f)
    }
}
impl IntoTrackableError<std::time::SystemTimeError> for ErrorKind {
    fn into_trackable_error(f: std::time::SystemTimeError) -> Error {
        ErrorKind::Other.cause(f)
    }
}
