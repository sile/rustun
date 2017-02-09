extern crate clap;
extern crate fibers;
extern crate futures;
extern crate rustun;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor};
use fibers::net::UdpSocket;
use futures::Future;
use rustun::transport::UdpChannel;
use rustun::client::Client;
use rustun::rfc5389;
use rustun::message::Message;

fn main() {
    let matches = App::new("rustun_cli")
        .arg(Arg::with_name("HOST")
            .index(1)
            .required(true))
        .arg(Arg::with_name("PORT")
            .short("p")
            .long("port")
            .takes_value(true)
            .required(true)
            .default_value("3478"))
        .get_matches();

    let host = matches.value_of("HOST").unwrap();
    let port = matches.value_of("PORT").unwrap();
    let addr = format!("{}:{}", host, port).parse().expect("Invalid UDP address");

    let mut executor = InPlaceExecutor::new().unwrap();
    let future =
        UdpSocket::bind("0.0.0.0:0".parse().unwrap()).map_err(From::from).and_then(move |socket| {
            let channel = UdpChannel::new(socket, addr);
            let client: Client<_, rfc5389::Attribute, _> = Client::new(channel);
            let request = Message::request(rfc5389::Method::Binding);
            client.call(request).map(|(_, m)| m)
        });
    let monitor = executor.spawn_monitor(future);
    let result = executor.run_fiber(monitor).unwrap();
    println!("RESULT: {:?}", result);
}
