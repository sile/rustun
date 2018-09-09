use bytecodec;
use fibers::sync::oneshot::MonitorError;
use std::io;
use std::sync::mpsc::SendError;
use stun_codec::AttributeType;
use trackable::error::{self, ErrorKindExt, TrackableError};

/// This crate specific `Error` type.
#[derive(Debug, Clone)]
pub struct Error(TrackableError<ErrorKind>);
derive_traits_for_trackable_error_newtype!(Error, ErrorKind);
impl From<MonitorError<Error>> for Error {
    fn from(f: MonitorError<Error>) -> Self {
        f.unwrap_or_else(|| {
            ErrorKind::Other
                .cause("Monitor channel has disconnected")
                .into()
        })
    }
}
impl From<io::Error> for Error {
    fn from(f: io::Error) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}
impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        ErrorKind::Other.cause("Receiver has terminated").into()
    }
}
impl From<bytecodec::Error> for Error {
    fn from(f: bytecodec::Error) -> Self {
        let bytecodec_error_kind = *f.kind();
        let kind = match bytecodec_error_kind {
            bytecodec::ErrorKind::InvalidInput => ErrorKind::InvalidInput,
            _ => ErrorKind::Other,
        };
        track!(kind.takes_over(f); bytecodec_error_kind).into()
    }
}
impl From<MessageError> for Error {
    fn from(f: MessageError) -> Self {
        ErrorKind::InvalidMessage(f.kind().clone())
            .takes_over(f)
            .into()
    }
}

/// Possible error kinds.
#[derive(Debug, Clone)]
pub enum ErrorKind {
    InvalidInput,
    InvalidMessage(MessageErrorKind),
    Other,
}
impl error::ErrorKind for ErrorKind {}

/// Message level error.
#[derive(Debug, Clone)]
pub struct MessageError(TrackableError<MessageErrorKind>);
derive_traits_for_trackable_error_newtype!(MessageError, MessageErrorKind);
impl From<MonitorError<MessageError>> for MessageError {
    fn from(f: MonitorError<MessageError>) -> Self {
        f.unwrap_or_else(|| {
            MessageErrorKind::Other
                .cause("`Channel` instance has dropped")
                .into()
        })
    }
}
impl From<Error> for MessageError {
    fn from(f: Error) -> Self {
        let original_error_kind = f.kind().clone();
        track!(MessageErrorKind::Other.takes_over(f); original_error_kind).into()
    }
}

/// Possible message-level error kinds.
#[derive(Debug, Clone)]
pub enum MessageErrorKind {
    // InvalidResponse
    UnknownTransaction,
    UnexpectedMethod,

    // InvalidRequest
    TransactionIdConflict,

    UnknownAttributes(Vec<AttributeType>),
    MalformedAttribute,
    UnexpectedClass,
    Timeout,
    InvalidInput,
    Other,
}
impl error::ErrorKind for MessageErrorKind {}
