extern crate clap;
extern crate fibers_global;
extern crate rustun;
extern crate stun_codec;
#[macro_use]
extern crate trackable;

use clap::{App, Arg};
use rustun::message::{Request, Response, SuccessResponse};
use rustun::server::{Action, HandleMessage, UdpServer};
use std::net::SocketAddr;
use stun_codec::rfc5389;
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
        )
        .get_matches();

    let port = matches.value_of("PORT").unwrap();
    let addr = track_any_err!(format!("0.0.0.0:{}", port).parse())?;

    let server = UdpServer::start(fibers_global::handle(), addr, BindingHandler);
    track!(fibers_global::execute(server))?;
    Ok(())
}

struct BindingHandler;
impl HandleMessage for BindingHandler {
    type Attribute = rfc5389::Attribute;

    fn handle_call(
        &mut self,
        peer: SocketAddr,
        request: Request<Self::Attribute>,
    ) -> Action<Response<Self::Attribute>> {
        // TODO: check method
        let mut response = SuccessResponse::new(request);
        response.push_attribute(rfc5389::attributes::XorMappedAddress::new(peer).into());
        Action::Reply(Ok(response))
    }
}
