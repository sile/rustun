extern crate clap;
extern crate fibers;
extern crate rustun;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor, Spawn};
use rustun::client::UdpClient;
use rustun::rfc5389;
use rustun::{Client, Method};

fn main() {
    let matches = App::new("rustun_cli")
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
    let addr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid UDP address");

    let mut executor = InPlaceExecutor::new().unwrap();
    let mut client = UdpClient::new(&executor.handle(), addr);
    let request = rfc5389::methods::Binding.request::<rfc5389::Attribute>();
    let monitor = executor.spawn_monitor(client.call(request));
    match executor.run_fiber(monitor).unwrap() {
        Ok(v) => println!("OK: {:?}", v),
        Err(e) => println!("ERROR: {}", e),
    }
}
