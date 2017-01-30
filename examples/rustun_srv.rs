extern crate clap;
extern crate fibers;
extern crate futures;
extern crate rustun;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor};
use futures::Future;
use rustun::server::StunServer;

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
    let future = StunServer::start(addr)
        .and_then(|server| server.socket.recv_from(vec![0; 1024]).map_err(|(_, _, e)| e))
        .map(|(_, mut buf, size, _)| {
            buf.truncate(size);
            buf
        });
    let monitor = executor.spawn_monitor(future);
    let result = executor.run_fiber(monitor).unwrap();
    println!("RESULT: {:?}", result);
}
