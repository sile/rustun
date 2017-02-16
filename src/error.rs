use std::io;
use std::fmt;
use std::error;
use failure::{Failure, MaybeFailure};
use fibers::sync::oneshot::MonitorError;

type BoxError = Box<error::Error + Send + Sync>;

#[derive(Debug)]
pub enum Error {
    Timeout,
    Full,
    NotStunMessage(BoxError),
    Unsupported(BoxError),
    Other(BoxError),
    Failed(Failure),
}
impl Error {
    pub fn failed<E>(error: E) -> Self
        where E: Into<BoxError>
    {
        Error::from(Failure::new(error))
    }
    pub fn unsupported<E>(error: E) -> Self
        where E: Into<BoxError>
    {
        Error::Unsupported(error.into())
    }
    pub fn not_stun<E>(error: E) -> Self
        where E: Into<BoxError>
    {
        Error::NotStunMessage(error.into())
    }
    pub fn other<E>(error: E) -> Self
        where E: Into<BoxError>
    {
        Error::Other(error.into())
    }
    pub fn get<T: error::Error + 'static>(&self) -> Option<&T> {
        match *self {
            Error::NotStunMessage(ref e) => e.downcast_ref(),
            Error::Unsupported(ref e) => e.downcast_ref(),
            Error::Other(ref e) => e.downcast_ref(),
            Error::Failed(ref e) => e.reason().downcast_ref(),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Timeout => write!(f, "Timeout"),
            Error::Full => write!(f, "Over capacity"),
            Error::NotStunMessage(ref e) => write!(f, "Not STUN message: {}", e),
            Error::Unsupported(ref e) => write!(f, "Unsupported feature: {}", e),
            Error::Other(ref e) => write!(f, "{}", e),
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
            Error::Other(ref e) => e.description(),
            Error::Failed(_) => "Failed",
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::NotStunMessage(ref e) => e.cause(),
            Error::Unsupported(ref e) => e.cause(),
            Error::Other(ref e) => e.cause(),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn downcast_works() {
        use std::io;

        let inner = io::Error::new(io::ErrorKind::Other, "other");
        let e = Error::other(inner);
        assert!(e.get::<io::Error>().is_some());
        assert!(e.get::<Error>().is_none());

        let inner = io::Error::new(io::ErrorKind::Other, "other");
        let e = Error::failed(inner);
        assert!(e.get::<io::Error>().is_some());
        assert!(e.get::<Error>().is_none());

    }
}
