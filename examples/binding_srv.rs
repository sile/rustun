extern crate clap;
extern crate fibers_global;
extern crate rustun;
extern crate stun_codec;
#[macro_use]
extern crate trackable;

use clap::{App, Arg};
use rustun::server::{BindingHandler, UdpServer};
use trackable::error::MainError;

fn main() -> Result<(), MainError> {
    let matches = App::new("binding_srv")
        .arg(
            Arg::with_name("PORT")
                .short("p")
                .long("port")
                .takes_value(true)
                .required(true)
                .default_value("3478"),
        ).get_matches();

    let port = matches.value_of("PORT").unwrap();
    let addr = track_any_err!(format!("0.0.0.0:{}", port).parse())?;

    let server = track!(fibers_global::execute(UdpServer::start(
        fibers_global::handle(),
        addr,
        BindingHandler
    )))?;
    track!(fibers_global::execute(server))?;
    Ok(())
}
