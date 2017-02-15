pub use client::BoxClient;
pub use self::base::BaseClient;
pub use self::tcp::TcpClient;
pub use self::udp::UdpClient;
pub use self::rate_limited::RateLimitedClient;

pub mod futures {
    pub use super::base::{BaseCall, BaseCast};
    pub use super::tcp::{TcpCall, TcpCast};
    pub use super::udp::{UdpCall, UdpCast};
    pub use super::rate_limited::RateLimited;
}

mod base;
mod tcp;
mod udp;
mod rate_limited;
