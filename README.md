rustun
======

[![Crates.io: rustun](http://meritbadge.herokuapp.com/rustun)](https://crates.io/crates/rustun)
[![Build Status](https://travis-ci.org/sile/rustun.svg?branch=master)](https://travis-ci.org/sile/rustun)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An asynchronous implementation of STUN [[RFC 5389](https://tools.ietf.org/html/rfc5389)]
server and client written in Rust.

[Documentation](https://docs.rs/rustun)

Installation
------------

Add following lines to your `Cargo.toml`:

```toml
[dependencies]
rustun = "0.1"
```
# Examples

A client-side example that issues a Binding request:

```rust
extern crate fibers;
extern crate rustun;

use fibers::{Executor, InPlaceExecutor, Spawn};
use rustun::{Method, Client};
use rustun::client::UdpClient;
use rustun::rfc5389;

fn main() {
    let server_addr = "127.0.0.1:3478".parse().unwrap();
    let mut executor = InPlaceExecutor::new().unwrap();

    let mut client = UdpClient::new(&executor.handle(), server_addr);
    let request = rfc5389::methods::Binding.request::<rfc5389::Attribute>();
    let future = client.call(request);

    let monitor = executor.spawn_monitor(future);
    match executor.run_fiber(monitor).unwrap() {
        Ok(v) => println!("SUCCEEDE: {:?}", v),
        Err(e) => println!("ERROR: {}", e),
    }
}
