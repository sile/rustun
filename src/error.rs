use std::io;
use std::fmt;
use std::error;
use failure::{Failure, MaybeFailure};
use fibers::sync::oneshot::MonitorError;

#[derive(Debug)]
pub enum Error {
    Timeout,
    Full,
    NotStunMessage(String),
    Unsupported(String),
    Failed(Failure),
}
impl Error {
    pub fn failed<E>(error: E) -> Self
        where E: Into<Box<error::Error + Send + Sync>>
    {
        Error::from(Failure::new(error))
    }
    pub fn unsupported<T: Into<String>>(message: T) -> Self {
        Error::Unsupported(message.into())
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Timeout => write!(f, "Timeout"),
            Error::Full => write!(f, "Over capacity"),
            Error::NotStunMessage(ref s) => write!(f, "Not STUN message: {}", s),
            Error::Unsupported(ref s) => write!(f, "Unsupported feature: {}", s),
            Error::Failed(ref failure) => write!(f, "{}", failure),
        }
    }
}
impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Timeout => "Timeout",
            Error::Full => "Over capacity",
            Error::NotStunMessage(_) => "Not STUN message",
            Error::Unsupported(_) => "Unsupported feature",
            Error::Failed(_) => "Failed",
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Failed(ref e) => e.cause(),
            _ => None,
        }
    }
}
impl From<Failure> for Error {
    fn from(f: Failure) -> Self {
        Error::Failed(f)
    }
}
impl From<io::Error> for Error {
    fn from(f: io::Error) -> Self {
        Error::Failed(Failure::new(f))
    }
}
impl From<MonitorError<Error>> for Error {
    fn from(f: MonitorError<Error>) -> Self {
        f.unwrap_or_else(|| Error::failed("Monitor channel disconnected"))
    }
}
impl MaybeFailure for Error {
    fn try_as_failure_mut(&mut self) -> Option<&mut Failure> {
        if let Error::Failed(ref mut f) = *self {
            Some(f)
        } else {
            None
        }
    }
    fn try_into_failure(self) -> Result<Failure, Self> {
        if let Error::Failed(f) = self {
            Ok(f)
        } else {
            Err(self)
        }
    }
}
