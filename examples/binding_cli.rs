extern crate clap;
extern crate fibers_global;
extern crate futures;
extern crate rustun;
extern crate stun_codec;
#[macro_use]
extern crate trackable;

use clap::{App, Arg};
use futures::Future;
use rustun::channel::Channel;
use rustun::client::Client;
use rustun::message::Request;
use rustun::transport::{RetransmitTransporter, StunUdpTransporter, UdpTransporter};
use std::net::ToSocketAddrs;
use stun_codec::rfc5389;
use trackable::error::Failed;
use trackable::error::MainError;

fn main() -> Result<(), MainError> {
    let matches = App::new("binding_cli")
        .arg(Arg::with_name("HOST").index(1).required(true))
        .arg(
            Arg::with_name("PORT")
                .short("p")
                .long("port")
                .takes_value(true)
                .required(true)
                .default_value("3478"),
        )
        .get_matches();

    let host = matches.value_of("HOST").unwrap();
    let port = matches.value_of("PORT").unwrap();
    let peer_addr = track_assert_some!(
        track_any_err!(format!("{}:{}", host, port).to_socket_addrs())?
            .filter(|x| x.is_ipv4())
            .nth(0),
        Failed
    );

    let local_addr = "0.0.0.0:0".parse().unwrap();
    let response = UdpTransporter::bind(local_addr)
        .map(RetransmitTransporter::new)
        .map(Channel::new)
        .and_then(move |channel: Channel<_, StunUdpTransporter<_>>| {
            let client = Client::new(&fibers_global::handle(), channel);
            let request = Request::<rfc5389::Attribute>::new(rfc5389::methods::BINDING);
            client.call(peer_addr, request)
        });
    let response = track!(fibers_global::execute(response))?;
    println!("{:?}", response);
    Ok(())
}
