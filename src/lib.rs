#[macro_use]
extern crate slog;
extern crate rand;
extern crate fibers;
extern crate futures;
#[macro_use]
extern crate trackable;
extern crate handy_async;

pub use error::{Error, ErrorKind};
pub use client::Client;
pub use server::HandleMessage;
pub use method::Method;
pub use message::Message;
pub use attribute::Attribute;

pub mod types;
pub mod clients;
pub mod servers;
pub mod message;
pub mod transport;
pub mod attribute;
pub mod constants;
pub mod rfc5389;

mod error;
mod method;
mod client;
mod server;

pub type Result<T> = ::std::result::Result<T, Error>;
