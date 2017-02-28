use std;
use std::io;
use std::sync::mpsc::RecvError;
use trackable::error::{self, IntoTrackableError, TrackableError, ErrorKindExt};
use fibers::sync::oneshot::MonitorError;

/// The error type for this crate.
pub type Error = TrackableError<ErrorKind>;

/// A list of error kind.
#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    /// The operation timed out.
    Timeout,

    /// The target resource is full (maybe temporary).
    Full,

    /// The input is invalid.
    Invalid,

    /// The input is valid, but requires unsupported features by this agent.
    Unsupported,

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
            ErrorKind::Other => "Some error happened",
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
