use std::io;
use std::fmt;
use std::error;
use std::any::Any;
use failure::{Failure, MaybeFailure};
use fibers::sync::oneshot::MonitorError;

pub enum Error {
    Timeout,
    Full,
    NotStunMessage(String),
    Unsupported(String),
    Other(String, Box<Any + Send + Sync>),
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
    pub fn other<E>(other: E) -> Self
        where E: fmt::Display + Any + Send + Sync
    {
        Error::Other(other.to_string(), Box::new(other))
    }
    pub fn get<T: Any>(&self) -> Option<&T> {
        use std::ops::Deref;
        if let Error::Other(_, ref e) = *self {
            let e: &Any = e.deref();
            e.downcast_ref()
        } else {
            None
        }
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Timeout => write!(f, "Timeout"),
            Error::Full => write!(f, "Full"),
            Error::NotStunMessage(ref s) => write!(f, "NotStunMessage({:?})", s),
            Error::Unsupported(ref s) => write!(f, "Unsupported({:?})", s),
            Error::Other(ref e, _) => write!(f, "Other({:?}, _)", e),
            Error::Failed(ref failure) => write!(f, "Failed({:?})", failure),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Timeout => write!(f, "Timeout"),
            Error::Full => write!(f, "Over capacity"),
            Error::NotStunMessage(ref s) => write!(f, "Not STUN message: {}", s),
            Error::Unsupported(ref s) => write!(f, "Unsupported feature: {}", s),
            Error::Other(ref e, _) => write!(f, "{}", e),
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
            Error::Other(ref e, _) => e,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn downcast_works() {
        use std::io;
        let inner = io::Error::new(io::ErrorKind::Other, "other");
        let e = Error::other(inner);
        assert!(e.get::<io::Error>().is_some());
    }
}
