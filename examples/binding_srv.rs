extern crate clap;
extern crate fibers;
extern crate rustun;
extern crate sloggers;

use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor, Spawn};
use rustun::rfc5389::handlers::BindingHandler;
use rustun::server::UdpServer;
use sloggers::terminal::TerminalLoggerBuilder;
use sloggers::Build;

fn main() {
    let matches = App::new("rustun_srv")
        .arg(
            Arg::with_name("PORT")
                .short("p")
                .long("port")
                .takes_value(true)
                .required(true)
                .default_value("3478"),
        )
        .get_matches();

    let port = matches.value_of("PORT").unwrap();
    let addr = format!("0.0.0.0:{}", port)
        .parse()
        .expect("Invalid UDP address");

    let logger = TerminalLoggerBuilder::new().build().unwrap();
    let mut executor = InPlaceExecutor::new().unwrap();
    let spawner = executor.handle();
    let monitor = executor.spawn_monitor(
        UdpServer::new(addr).start(spawner.boxed(), BindingHandler::with_logger(logger)),
    );
    let result = executor.run_fiber(monitor).unwrap();
    println!("RESULT: {:?}", result);
}
