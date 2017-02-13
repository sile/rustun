#[macro_use]
extern crate slog;
extern crate rand;
extern crate fibers;
extern crate futures;
#[macro_use]
extern crate failure;
extern crate byteorder;
extern crate handy_async;

pub use error::Error;
pub use method::Method;
pub use message::Message;
pub use attribute::Attribute;

pub mod io;
pub mod types;
pub mod message;
pub mod transport;
pub mod attribute;
pub mod constants;

mod error;
mod method;

pub type Result<T> = ::std::result::Result<T, Error>;
