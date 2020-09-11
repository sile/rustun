//! An asynchronous implementation of [STUN][RFC 5389] server and client.
//!
//! # Examples
//!
//! An example that issues a `BINDING` request:
//!
//! ```
//! # extern crate fibers_global;
//! # extern crate fibers_transport;
//! # extern crate futures;
//! # extern crate rustun;
//! # extern crate stun_codec;
//! # extern crate trackable;
//! use fibers_transport::UdpTransporter;
//! use futures::Future;
//! use rustun::channel::Channel;
//! use rustun::client::Client;
//! use rustun::message::Request;
//! use rustun::server::{BindingHandler, UdpServer};
//! use rustun::transport::StunUdpTransporter;
//! use rustun::Error;
//! use stun_codec::{rfc5389, MessageDecoder, MessageEncoder};
//!
//! # fn main() -> Result<(), trackable::error::MainError> {
//! let addr = "127.0.0.1:0".parse().unwrap();
//!
//! // Starts UDP server
//! let server = fibers_global::execute(UdpServer::start(fibers_global::handle(), addr, BindingHandler))?;
//! let server_addr = server.local_addr();
//! fibers_global::spawn(server.map(|_| ()).map_err(|e| panic!("{}", e)));
//!
//! // Sents BINDING request
//! let response = UdpTransporter::<MessageEncoder<_>, MessageDecoder<_>>::bind(addr)
//!     .map_err(Error::from)
//!     .map(StunUdpTransporter::new)
//!     .map(Channel::new)
//!     .and_then(move |channel| {
//!         let client = Client::new(&fibers_global::handle(), channel);
//!         let request = Request::<rfc5389::Attribute>::new(rfc5389::methods::BINDING);
//!         client.call(server_addr, request)
//!     });
//!
//! // Waits BINDING response
//! let response = fibers_global::execute(response)?;
//! assert!(response.is_ok());
//! # Ok(())
//! # }
//! ```
//!
//! You can run example server and client that handle `BINDING` method as follows:
//!
//! ```console
//! // Starts the STUN server in a shell.
//! $ cargo run --example binding_srv
//!
//! // Executes a STUN client in another shell.
//! $ cargo run --example binding_cli -- 127.0.0.1
//! Ok(SuccessResponse(Message {
//!     class: SuccessResponse,
//!     method: Method(1),
//!     transaction_id: TransactionId(0x344A403694972F5E53B69465),
//!     attributes: [Known { inner: XorMappedAddress(XorMappedAddress(V4(127.0.0.1:54754))),
//!                          padding: Some(Padding([])) }]
//! }))
//! ```
//!
//! # References
//!
//! - [RFC 5389 - Session Traversal Utilities for NAT (STUN)][RFC 5389]
//!
//! [RFC 5389]: https://tools.ietf.org/html/rfc5389
#![warn(missing_docs)]
extern crate bytecodec;
extern crate factory;
extern crate fibers;
#[cfg(test)]
extern crate fibers_global;
extern crate fibers_timeout_queue;
extern crate fibers_transport;
extern crate futures;
extern crate rand;
extern crate stun_codec;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

pub mod channel;
pub mod client;
pub mod message;
pub mod server;
pub mod transport;

mod error;

/// A specialized `Result` type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use crate::channel::Channel;
    use crate::client::Client;
    use crate::message::Request;
    use crate::server::{BindingHandler, TcpServer, UdpServer};
    use crate::transport::{StunTcpTransporter, StunUdpTransporter};
    use crate::Error;
    use factory::DefaultFactory;
    use fibers_global;
    use fibers_transport::{TcpTransporter, UdpTransporter};
    use futures::Future;
    use std::thread;
    use std::time::Duration;
    use stun_codec::rfc5389;
    use stun_codec::{MessageDecoder, MessageEncoder};
    use trackable::error::MainError;

    #[test]
    fn basic_udp_test() -> Result<(), MainError> {
        let server = fibers_global::execute(UdpServer::start(
            fibers_global::handle(),
            "127.0.0.1:0".parse().unwrap(),
            BindingHandler,
        ))?;
        let server_addr = server.local_addr();
        fibers_global::spawn(server.map(|_| ()).map_err(|e| panic!("{}", e)));

        let client_addr = "127.0.0.1:0".parse().unwrap();
        let response = UdpTransporter::<MessageEncoder<_>, MessageDecoder<_>>::bind(client_addr)
            .map_err(Error::from)
            .map(StunUdpTransporter::new)
            .map(Channel::new)
            .and_then(move |channel| {
                let client = Client::new(&fibers_global::handle(), channel);
                let request = Request::<rfc5389::Attribute>::new(rfc5389::methods::BINDING);
                client.call(server_addr, request)
            });
        let response = track!(fibers_global::execute(response))?;
        assert!(response.is_ok());

        Ok(())
    }

    #[test]
    fn basic_tcp_test() -> Result<(), MainError> {
        let server = fibers_global::execute(TcpServer::start(
            fibers_global::handle(),
            "127.0.0.1:0".parse().unwrap(),
            DefaultFactory::<BindingHandler>::new(),
        ))?;
        let server_addr = server.local_addr();

        fibers_global::spawn(server.map(|_| ()).map_err(|e| panic!("{}", e)));
        thread::sleep(Duration::from_millis(50));

        let response = TcpTransporter::<MessageEncoder<_>, MessageDecoder<_>>::connect(server_addr)
            .map_err(Error::from)
            .map(StunTcpTransporter::new)
            .map(Channel::new)
            .and_then(move |channel| {
                let client = Client::new(&fibers_global::handle(), channel);
                let request = Request::<rfc5389::Attribute>::new(rfc5389::methods::BINDING);
                client.call((), request)
            });
        let response = track!(fibers_global::execute(response))?;
        assert!(response.is_ok());

        Ok(())
    }
}
