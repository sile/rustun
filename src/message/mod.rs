//! STUN message related components.
//!
//! > STUN is a client-server protocol.  It supports two types of
//! > transactions. One is a **request/response** transaction in which a
//! > client sends a **request** to a server, and the server returns a
//! > **response**. The second is an **indication** transaction in which either
//! > agent -- client or server -- sends an indication that generates no
//! > response. Both types of transactions include a transaction ID, which
//! > is a randomly selected 96-bit number.  For **request/response**
//! > transactions, this transaction ID allows the client to associate the
//! > response with the request that generated it; for indications, the
//! > transaction ID serves as a debugging aid.
//! >
//! > All STUN messages start with a fixed header that includes a method, a
//! > class, and the transaction ID.  The method indicates which of the
//! > various requests or indications this is; this specification defines
//! > just one method, Binding, but other methods are expected to be
//! > defined in other documents.  The class indicates whether this is a
//! > **request**, a **success response**, an **error response**, or an **indication**.
//! > Following the fixed header comes zero or more attributes, which are
//! > Type-Length-Value extensions that convey additional information for
//! > the specific message.
//! >
//! > [RFC 5389 -- 3. Overview of Operation](https://tools.ietf.org/html/rfc5389#section-3)
use rand;

use {Method, Attribute};
use types::TransactionId;
use method::{Requestable, Indicatable};

pub use self::raw::{RawMessage, Class};

mod raw;

/// Response message.
pub type Response<M, A> = Result<SuccessResponse<M, A>, ErrorResponse<M, A>>;

/// Request message.
#[derive(Debug, Clone)]
pub struct Request<M, A> {
    method: M,
    transaction_id: TransactionId,
    attributes: Vec<A>,
}
impl<M, A> Request<M, A>
    where M: Method,
          A: Attribute
{
    /// Makes a new request message.
    pub fn new(method: M) -> Self
        where M: Requestable
    {
        Request {
            method: method,
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }

    /// Returns the method of this message.
    pub fn method(&self) -> &M {
        &self.method
    }

    /// Returns the transaction ID of this message.
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }

    /// Returns the attributes of this message.
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }

    /// Converts into a success response message.
    pub fn into_success_response(self) -> SuccessResponse<M, A> {
        SuccessResponse::new(self.method, self.transaction_id)
    }

    /// Converts into an error response message.
    pub fn into_error_response(self) -> ErrorResponse<M, A> {
        ErrorResponse::new(self.method, self.transaction_id)
    }
}

/// Indication message.
#[derive(Debug, Clone)]
pub struct Indication<M, A> {
    method: M,
    transaction_id: TransactionId,
    attributes: Vec<A>,
}
impl<M, A> Indication<M, A>
    where M: Method,
          A: Attribute
{
    /// Makes a new indication message.
    pub fn new(method: M) -> Self
        where M: Indicatable
    {
        Indication {
            method: method,
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }

    /// Returns the method of this message.
    pub fn method(&self) -> &M {
        &self.method
    }

    /// Returns the transaction ID of this message.
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }

    /// Returns the attributes of this message.
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }
}

/// Success response message.
///
/// This is usually created by calling `Request::into_success_response` method.
#[derive(Debug, Clone)]
pub struct SuccessResponse<M, A> {
    method: M,
    transaction_id: TransactionId,
    attributes: Vec<A>,
}
impl<M, A> SuccessResponse<M, A>
    where M: Method,
          A: Attribute
{
    fn new(method: M, transaction_id: TransactionId) -> Self {
        SuccessResponse {
            method: method,
            transaction_id: transaction_id,
            attributes: Vec::new(),
        }
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }

    /// Returns the method of this message.
    pub fn method(&self) -> &M {
        &self.method
    }

    /// Returns the transaction ID of this message.
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }

    /// Returns the attributes of this message.
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }
}

/// Error response message.
///
/// This is usually created by calling `Request::into_error_response` method.
#[derive(Debug, Clone)]
pub struct ErrorResponse<M, A> {
    method: M,
    transaction_id: TransactionId,
    attributes: Vec<A>,
}
impl<M, A> ErrorResponse<M, A>
    where M: Method,
          A: Attribute
{
    fn new(method: M, transaction_id: TransactionId) -> Self {
        ErrorResponse {
            method: method,
            transaction_id: transaction_id,
            attributes: Vec::new(),
        }
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }

    /// Adds `attribute` to the tail of the attributes of this message.
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }

    /// Returns the method of this message.
    pub fn method(&self) -> &M {
        &self.method
    }

    /// Returns the transaction ID of this message.
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }

    /// Returns the attributes of this message.
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }
}
