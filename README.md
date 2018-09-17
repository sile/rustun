rustun
======

[![Crates.io: rustun](https://img.shields.io/crates/v/rustun.svg)](https://crates.io/crates/rustun)
[![Documentation](https://docs.rs/rustun/badge.svg)](https://docs.rs/rustun)
[![Build Status](https://travis-ci.org/sile/rustun.svg?branch=master)](https://travis-ci.org/sile/rustun)
[![Code Coverage](https://codecov.io/gh/sile/rustun/branch/master/graph/badge.svg)](https://codecov.io/gh/sile/rustun/branch/master)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust library for implementing STUN server and client asynchronously.

[Documentation](https://docs.rs/rustun)

The STUN protocol is defined in [RFC 5389](https://tools.ietf.org/html/rfc5389).

Examples
--------

An example that issues a `BINDING` request:


```rust
use fibers_transport::UdpTransporter;
use futures::Future;
use rustun::channel::Channel;
use rustun::client::Client;
use rustun::message::Request;
use rustun::server::{BindingHandler, UdpServer};
use rustun::transport::StunUdpTransporter;
use rustun::Error;
use stun_codec::{rfc5389, MessageDecoder, MessageEncoder};

let addr = "127.0.0.1:0".parse().unwrap();

// Starts UDP server
let server = fibers_global::execute(UdpServer::start(fibers_global::handle(), addr, BindingHandler))?;
let server_addr = server.local_addr();
fibers_global::spawn(server.map(|_| ()).map_err(|e| panic!("{}", e)));

// Sents BINDING request
let response = UdpTransporter::<MessageEncoder<_>, MessageDecoder<_>>::bind(addr)
    .map_err(Error::from)
    .map(StunUdpTransporter::new)
    .map(Channel::new)
    .and_then(move |channel| {
        let client = Client::new(&fibers_global::handle(), channel);
        let request = Request::<rfc5389::Attribute>::new(rfc5389::methods::BINDING);
        client.call(server_addr, request)
    });

// Waits BINDING response
let response = fibers_global::execute(response)?;
assert!(response.is_ok());
```

You can run example server and client which handle `Binding` method as follows:

```console
// Starts the STUN server in a shell.
$ cargo run --example binding_srv

// Executes a STUN client in another shell.
$ cargo run --example binding_cli -- 127.0.0.1
Ok(SuccessResponse(Message {
    class: SuccessResponse,
    method: Method(1),
    transaction_id: TransactionId(0x344A403694972F5E53B69465),
    attributes: [Known { inner: XorMappedAddress(XorMappedAddress(V4(127.0.0.1:54754))),
                         padding: Some(Padding([])) }]
}))
```
