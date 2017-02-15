extern crate clap;
extern crate fibers;
extern crate futures;
extern crate rustun;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor, Spawn};
use futures::Future;
use rustun::servers::UdpServer;

fn main() {
    let matches = App::new("rustun_srv")
        .arg(Arg::with_name("PORT")
            .short("p")
            .long("port")
            .takes_value(true)
            .required(true)
            .default_value("3478"))
        .get_matches();

    let port = matches.value_of("PORT").unwrap();
    let addr = format!("0.0.0.0:{}", port).parse().expect("Invalid UDP address");

    let mut executor = InPlaceExecutor::new().unwrap();
    let link = executor.spawn_link(UdpServer::new(addr).start());
    let result = executor.run_future(link).unwrap();
    println!("RESULT: {:?}", result);
}
