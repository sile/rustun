extern crate clap;
extern crate fibers;
extern crate futures;
extern crate rustun;
#[macro_use]
extern crate track_err;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor, Spawn};
use fibers::net::UdpSocket;
use futures::Future;
use track_err::ErrorKindExt;
use rustun::{Method, Client, ErrorKind};
use rustun::rfc5389::{self, UdpClient};

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
    let handle = executor.handle();
    let future = UdpSocket::bind("0.0.0.0:0".parse().unwrap())
        .map_err(|e| track_err!(ErrorKind::Failed.cause(e)))
        .and_then(move |socket| {
            let mut client = UdpClient::new(handle, socket, addr);
            let request = rfc5389::Method::binding().request();
            client.call(request).map_err(|e| track_err!(e))
        });
    let monitor = executor.spawn_monitor(future);
    match executor.run_fiber(monitor).unwrap() {
        Ok(v) => println!("SUCCEEDE: {:?}", v),
        Err(e) => println!("ERROR: {}", e),
    }
}
