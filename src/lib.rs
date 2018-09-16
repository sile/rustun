//! An asynchronous implementation of [STUN][RFC 5389] server and client.
//!
//! # Examples
//!
//! An example that issues a `BINDING` request:
//!
//! ```
//! # extern crate fibers_global;
//! # extern crate futures;
//! # extern crate rustun;
//! # extern crate stun_codec;
//! # extern crate trackable;
//! use futures::Future;
//! use rustun::channel::Channel;
//! use rustun::client::Client;
//! use rustun::message::Request;
//! use rustun::server::{BindingHandler, UdpServer};
//! use rustun::transport::{RetransmitTransporter, UdpTransporter, StunUdpTransporter};
//! use stun_codec::rfc5389;
//!
//! # fn main() -> Result<(), trackable::error::MainError> {
//! let server_addr = "127.0.0.1:3478".parse().unwrap();
//! let client_addr = "127.0.0.1:0".parse().unwrap();
//!
//! // Starts UDP server
//! let server = UdpServer::start(fibers_global::handle(), server_addr, BindingHandler);
//! fibers_global::spawn(server.map(|_| ()).map_err(|e| panic!("{}", e)));
//!
//! // Sents BINDING request
//! let response = UdpTransporter::bind(client_addr)
//!     .map(RetransmitTransporter::new)
//!     .map(Channel::new)
//!     .and_then(move |channel: Channel<_, StunUdpTransporter<_>>| {
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
// #![warn(missing_docs)] // TODO
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
    use factory::DefaultFactory;
    use fibers_global;
    use futures::Future;
    use std::thread;
    use std::time::Duration;
    use stun_codec::rfc5389;
    use trackable::error::MainError;

    use channel::Channel;
    use client::Client;
    use message::Request;
    use server::{BindingHandler, TcpServer, UdpServer};
    use transport::{RetransmitTransporter, StunUdpTransporter, TcpTransporter, UdpTransporter};

    #[test]
    fn basic_udp_test() -> Result<(), MainError> {
        let server_addr = "127.0.0.1:3479".parse().unwrap();
        let client_addr = "127.0.0.1:0".parse().unwrap();

        let server = UdpServer::start(fibers_global::handle(), server_addr, BindingHandler);
        fibers_global::spawn(server.map(|_| ()).map_err(|e| panic!("{}", e)));

        let response = UdpTransporter::bind(client_addr)
            .map(RetransmitTransporter::new)
            .map(Channel::new)
            .and_then(move |channel: Channel<_, StunUdpTransporter<_>>| {
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
        let server_addr = "127.0.0.1:3480".parse().unwrap();

        let server = TcpServer::start(
            fibers_global::handle(),
            server_addr,
            DefaultFactory::<BindingHandler>::new(),
        );
        fibers_global::spawn(server.map(|_| ()).map_err(|e| panic!("{}", e)));
        thread::sleep(Duration::from_millis(50));

        let response = TcpTransporter::connect(server_addr)
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
}
