#[macro_use]
extern crate trackable;

use clap::Parser;
use rustun::server::{BindingHandler, UdpServer};
use trackable::error::MainError;

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long, default_value_t = 3478)]
    port: u16,
}

fn main() -> Result<(), MainError> {
    let args = Args::parse();
    let addr = track_any_err!(format!("0.0.0.0:{}", args.port).parse())?;

    let server = track!(fibers_global::execute(UdpServer::start(
        fibers_global::handle(),
        addr,
        BindingHandler
    )))?;
    track!(fibers_global::execute(server))?;
    Ok(())
}
