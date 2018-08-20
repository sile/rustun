//! Individual definition of the methods that are defined in [RFC 5389]
//! (https://tools.ietf.org/html/rfc5389).
use method;
use types::U12;
use Method;

/// The codepoint of the [Binding](struct.Binding.html) method.
pub const METHOD_BINDING: u16 = 0x0001;

/// Binding method.
///
/// > This document defines a single method called Binding.  The Binding
/// > method can be used either in request/response transactions or in
/// > indication transactions.  When used in request/response transactions,
/// > the Binding method can be used to determine the particular "binding"
/// > a NAT has allocated to a STUN client.  When used in either request/
/// > response or in indication transactions, the Binding method can also
/// > be used to keep these "bindings" alive.
/// >
/// > In the Binding request/response transaction, a Binding request is
/// > sent from a STUN client to a STUN server.  When the Binding request
/// > arrives at the STUN server, it may have passed through one or more
/// > NATs between the STUN client and the STUN server (in Figure 1, there
/// > were two such NATs).  As the Binding request message passes through a
/// > NAT, the NAT will modify the source transport address (that is, the
/// > source IP address and the source port) of the packet.  As a result,
/// > the source transport address of the request received by the server
/// > will be the public IP address and port created by the NAT closest to
/// > the server.  This is called a reflexive transport address.  The STUN
/// > server copies that source transport address into an XOR-MAPPED-
/// > ADDRESS attribute in the STUN Binding response and sends the Binding
/// > response back to the STUN client.  As this packet passes back through
/// > a NAT, the NAT will modify the destination transport address in the
/// > IP header, but the transport address in the XOR-MAPPED-ADDRESS
/// > attribute within the body of the STUN response will remain untouched.
/// > In this way, the client can learn its reflexive transport address
/// > allocated by the outermost NAT with respect to the STUN server.
/// >
/// > [RFC 5389 -- 3. Overview of Operation](https://tools.ietf.org/html/rfc5389#section-3)
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Binding;
impl Method for Binding {
    fn from_u12(value: U12) -> Option<Self> {
        if value.as_u16() == METHOD_BINDING {
            Some(Binding)
        } else {
            None
        }
    }
    fn as_u12(&self) -> U12 {
        U12::from_u16(METHOD_BINDING).unwrap()
    }
}
impl method::Requestable for Binding {}
impl method::Indicatable for Binding {}
