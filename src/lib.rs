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
pub use attribute::Attribute;
pub use transport::Transport;

pub mod types;
// pub mod servers;
pub mod message;
pub mod transport;
pub mod attribute;
pub mod constants;
pub mod method;
pub mod rfc5389;

mod error;
pub mod client;
pub mod server;

pub type Result<T> = ::std::result::Result<T, Error>;
