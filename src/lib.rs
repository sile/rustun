//! Asynchronous implementation of STUN [[RFC 5389](https://tools.ietf.org/html/rfc5389)]
//! server and client.
//!
//! # Examples
//!
//! A client-side example that issues a Binding request:
//!
//! ```no_run
//! extern crate fibers;
//! extern crate rustun;
//!
//! use fibers::{Executor, InPlaceExecutor, Spawn};
//! use rustun::{Method, Client};
//! use rustun::client::UdpClient;
//! use rustun::rfc5389;
//!
//! fn main() {
//!     let server_addr = "127.0.0.1:3478".parse().unwrap();
//!     let mut executor = InPlaceExecutor::new().unwrap();
//!
//!     let mut client = UdpClient::new(&executor.handle(), server_addr);
//!     let request = rfc5389::methods::Binding.request::<rfc5389::Attribute>();
//!     let future = client.call(request);
//!
//!     let monitor = executor.spawn_monitor(future);
//!     match executor.run_fiber(monitor).unwrap() {
//!         Ok(v) => println!("OK: {:?}", v),
//!         Err(e) => println!("ERROR: {}", e),
//!     }
//! }
//! ```
//!
//! You can run example server and client which handle `Binding` method as follows:
//!
//! ```bash
//! # Starts the STUN server in a shell.
//! $ cargo run --example binding_srv
//!
//! # Executes a STUN client in another shell.
//! $ cargo run --example binding_cli -- 127.0.0.1
//! OK: Ok(SuccessResponse {
//!            method: Binding,
//!            transaction_id: [246, 217, 191, 180, 118, 246, 250, 168, 86, 124, 126, 130],
//!            attributes: [XorMappedAddress(XorMappedAddress(V4(127.0.0.1:61991)))]
//!       })
//! ```
// #![warn(missing_docs)] TODO

extern crate bytecodec;
extern crate factory;
extern crate fibers;
extern crate futures;
extern crate rand;
extern crate stun_codec;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

pub mod channel;
pub mod client;
pub mod constants;
pub mod message;
pub mod server;
pub mod transport;

mod error;
mod timeout_queue;

/// A specialized `Result` type for this crate.
pub type Result<T> = ::std::result::Result<T, Error>;
