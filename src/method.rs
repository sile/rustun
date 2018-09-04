//! Traits that are used to represent STUN method.
// use message::{Indication, Request};
use stun_codec::rfc5389::methods::Binding;
use stun_codec::Method;

/// This trait represents that the implementation method can be used in request messages.
pub trait RequestableMethod: Method {}

/// This trait represents that the implementation method can be used in indication messages.
pub trait IndicatableMethod: Method {}

impl RequestableMethod for Binding {}
impl IndicatableMethod for Binding {}
