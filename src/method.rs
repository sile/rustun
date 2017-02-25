//! Traits that are used to represent STUN method.
use Attribute;
use types::U12;
use message::{Request, Indication};

/// STUN method.
///
/// > All STUN messages start with a fixed header that includes a **method**, a
/// > class, and the transaction ID.  The **method** indicates which of the
/// > various requests or indications this is;
/// >
/// > [RFC 5389 -- 3. Overview of Operation](https://tools.ietf.org/html/rfc5389#section-3)
pub trait Method: Sized {
    /// Tries to convert from `codepoint` to the corresponding method.
    ///
    /// If no such method exists, this will return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustun::Method;
    /// use rustun::rfc5389::methods::Binding;
    /// use rustun::types::U12;
    ///
    /// assert!(Binding::from_u12(U12::from_u8(1)).is_some());
    /// assert!(Binding::from_u12(U12::from_u8(0)).is_none());
    /// ```
    fn from_u12(codepoint: U12) -> Option<Self>;

    /// Returns the codepoint corresponding this method.
    ///
    /// # Example
    ///
    /// ```
    /// use rustun::Method;
    /// use rustun::rfc5389::methods::Binding;
    /// use rustun::types::U12;
    ///
    /// assert_eq!(Binding.as_u12(), U12::from_u8(1));
    /// ```
    fn as_u12(&self) -> U12;

    /// Makes a request message which have this method.
    fn request<A>(self) -> Request<Self, A>
        where Self: Requestable,
              A: Attribute
    {
        Request::new(self)
    }

    /// Makes a indication message which have this method.
    fn indication<A>(self) -> Indication<Self, A>
        where Self: Indicatable,
              A: Attribute
    {
        Indication::new(self)
    }
}
impl Method for U12 {
    fn from_u12(codepoint: U12) -> Option<Self> {
        Some(codepoint)
    }
    fn as_u12(&self) -> U12 {
        *self
    }
}

/// This trait represents that the implementation method can be used in request messages.
pub trait Requestable: Method {}

/// This trait represents that the implementation method can be used in indication messages.
pub trait Indicatable: Method {}
