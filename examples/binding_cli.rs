#[macro_use]
extern crate trackable;

use clap::Parser;
use fibers_transport::UdpTransporter;
use futures::Future;
use rustun::channel::Channel;
use rustun::client::Client;
use rustun::message::Request;
use rustun::transport::StunUdpTransporter;
use rustun::Error;
use std::net::ToSocketAddrs;
use stun_codec::rfc5389;
use stun_codec::{MessageDecoder, MessageEncoder};
use trackable::error::Failed;
use trackable::error::MainError;

#[derive(Debug, Parser)]
struct Args {
    host: String,

    #[clap(short, long, default_value_t = 3478)]
    port: usize,
}

fn main() -> Result<(), MainError> {
    let args = Args::parse();
    let peer_addr = track_assert_some!(
        track_any_err!(format!("{}:{}", args.host, args.port).to_socket_addrs())?
            .filter(|x| x.is_ipv4())
            .nth(0),
        Failed
    );

    let local_addr = "0.0.0.0:0".parse().unwrap();
    let response = UdpTransporter::<MessageEncoder<_>, MessageDecoder<_>>::bind(local_addr)
        .map_err(Error::from)
        .map(StunUdpTransporter::new)
        .map(Channel::new)
        .and_then(move |channel| {
            let client = Client::new(&fibers_global::handle(), channel);
            let request = Request::<rfc5389::Attribute>::new(rfc5389::methods::BINDING);
            client.call(peer_addr, request)
        });
    let response = track!(fibers_global::execute(response))?;
    println!("{:?}", response);
    Ok(())
}
