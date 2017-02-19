use std::io;
use std::sync::mpsc::RecvError;
use trackable::error::{self, IntoTrackableError, TrackableError, ErrorKindExt};
use fibers::sync::oneshot::MonitorError;

pub type Error = TrackableError<ErrorKind>;

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Timeout,
    Full,
    NotStunMessage,
    Unsupported,
    Failed,
}
impl error::ErrorKind for ErrorKind {}
impl IntoTrackableError<MonitorError<Error>> for ErrorKind {
    fn into_trackable_error(f: MonitorError<Error>) -> Error {
        f.unwrap_or(ErrorKind::Failed.into())
    }
}
impl IntoTrackableError<io::Error> for ErrorKind {
    fn into_trackable_error(f: io::Error) -> Error {
        ErrorKind::Failed.cause(f)
    }
}
impl IntoTrackableError<RecvError> for ErrorKind {
    fn into_trackable_error(f: RecvError) -> Error {
        ErrorKind::Failed.cause(f)
    }
}
