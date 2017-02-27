extern crate clap;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate fibers;
extern crate rustun;

use clap::{App, Arg};
use slog::{Logger, DrainExt, Record, LevelFilter};
use fibers::{Executor, InPlaceExecutor, Spawn};
use rustun::server::UdpServer;
use rustun::rfc5389::handlers::BindingHandler;

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

    let place_fn = |info: &Record| format!("{}:{}", info.module(), info.line());
    let logger = Logger::root(LevelFilter::new(slog_term::streamer().build(), slog::Level::Info)
                                  .fuse(),
                              o!("place" => place_fn));

    let mut executor = InPlaceExecutor::new().unwrap();
    let spawner = executor.handle();
    let monitor = executor.spawn_monitor(UdpServer::new(addr)
        .start(spawner.boxed(), BindingHandler::with_logger(logger)));
    let result = executor.run_fiber(monitor).unwrap();
    println!("RESULT: {:?}", result);
}
