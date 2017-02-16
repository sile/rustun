use std::io;
use track_err;
use fibers::sync::oneshot::MonitorError;

pub type Error = track_err::Error<ErrorKind>;

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Timeout,
    Full,
    NotStunMessage,
    Unsupported,
    Failed,
}
impl track_err::ErrorKind for ErrorKind {}
impl<'a> From<&'a MonitorError<Error>> for ErrorKind {
    fn from(f: &'a MonitorError<Error>) -> Self {
        match *f {
            MonitorError::Failed(ref e) => *e.kind(),
            MonitorError::Aborted => ErrorKind::Failed,
        }
    }
}
impl<'a> From<&'a io::Error> for ErrorKind {
    fn from(_: &'a io::Error) -> Self {
        ErrorKind::Failed
    }
}
