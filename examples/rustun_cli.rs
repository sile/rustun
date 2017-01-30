extern crate clap;
extern crate fibers;
extern crate futures;
extern crate rustun;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor};
use rustun::client::StunClient;

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
    let client = StunClient::new();
    let monitor = executor.spawn_monitor(client.binding(addr));
    let result = executor.run_fiber(monitor).unwrap();
    println!("RESULT: {:?}", result);
}
