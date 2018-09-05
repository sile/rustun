extern crate clap;
extern crate fibers_global;
extern crate futures;
extern crate rustun;
extern crate stun_codec;

use clap::{App, Arg};
use futures::Future;
use rustun::client::{Client, UdpClient};
use rustun::message::Request;
use rustun::transport::UdpTransporter;
use stun_codec::rfc5389;

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

    let response =
        UdpTransporter::bind("0.0.0.0:0".parse().unwrap()).and_then(move |transporter| {
            let mut client = UdpClient::new(transporter, addr);
            let request = Request::<_, rfc5389::Attribute>::new(rfc5389::methods::Binding);
            let future = client.call(request);
            fibers_global::spawn(client.map(|_| ()).map_err(|e| panic!("{}", e)));
            future
        });
    match fibers_global::execute(response) {
        Ok(v) => println!("OK: {:?}", v),
        Err(e) => println!("ERROR: {}", e),
    }
}
