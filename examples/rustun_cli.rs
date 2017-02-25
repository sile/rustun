extern crate clap;
extern crate fibers;
extern crate futures;
extern crate rustun;
#[macro_use]
extern crate trackable;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor, Spawn};
use futures::Future;
use rustun::Client;
use rustun::rfc5389::{self, UdpClient};
use rustun::method::Requestable;

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
    let future = UdpClient::new(handle, addr).and_then(|mut client| {
        let request = rfc5389::methods::Binding.request::<rfc5389::Method, rfc5389::Attribute>();
        track_err!(client.call(request))
    });
    let monitor = executor.spawn_monitor(future);
    match executor.run_fiber(monitor).unwrap() {
        Ok(v) => println!("SUCCEEDE: {:?}", v),
        Err(e) => println!("ERROR: {}", e),
    }
}
