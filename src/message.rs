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
//! > [RFC 5389 -- 3. Overview of Operation]
//!
//! [RFC 5389 -- 3. Overview of Operation]: https://tools.ietf.org/html/rfc5389#section-3
use rand;
use std;
use stun_codec::rfc5389::attributes::ErrorCode;
use stun_codec::{Attribute, Message, MessageClass, Method, TransactionId};

use {ErrorKind, Result};

/// Response message.
pub type Response<M, A> = std::result::Result<SuccessResponse<M, A>, ErrorResponse<M, A>>;

/// Request message.
#[derive(Debug, Clone)]
pub struct Request<M, A>(Message<M, A>);
impl<M: Method, A: Attribute> Request<M, A> {
    /// Makes a new request message.
    pub fn new(method: M) -> Self {
        Request(Message::new(
            MessageClass::Request,
            method,
            TransactionId::new(rand::random()),
        ))
    }

    /// Converts `Message` to `Request`.
    ///
    /// # Errors
    ///
    /// If the class of the given message is not `MessageClass::Request`,
    /// this function will return an `ErrorKind::InvalidInput` error.
    ///
    /// And if the message contains some unknown comprehension-required attributes,
    /// this function will return an `ErrorKind::UnknownAttributes` error.
    pub fn from_message(message: Message<M, A>) -> Result<Self> {
        track_assert_eq!(
            message.class(),
            MessageClass::Request,
            ErrorKind::InvalidInput
        );
        track!(check_unknown_attributes(&message))?;
        Ok(Request(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> &M {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> &TransactionId {
        self.0.transaction_id()
    }

    /// Returns an iterator that iterates over the known attributes in the message.
    pub fn attributes(&self) -> impl Iterator<Item = &A> {
        self.0.attributes()
    }

    /// Adds the given attribute to the tail of the attributes in the message.
    pub fn push_attribute(&mut self, attribute: A) {
        self.0.push_attribute(attribute);
    }

    /// Takes ownership of this instance, and returns the internal message.
    pub fn into_message(self) -> Message<M, A> {
        self.0
    }
}
impl<M: Method, A: Attribute> AsRef<Message<M, A>> for Request<M, A> {
    fn as_ref(&self) -> &Message<M, A> {
        &self.0
    }
}
impl<M: Method, A: Attribute> AsMut<Message<M, A>> for Request<M, A> {
    fn as_mut(&mut self) -> &mut Message<M, A> {
        &mut self.0
    }
}

/// Indication message.
#[derive(Debug, Clone)]
pub struct Indication<M, A>(Message<M, A>);
impl<M: Method, A: Attribute> Indication<M, A> {
    /// Makes a new indication message.
    pub fn new(method: M) -> Self {
        Indication(Message::new(
            MessageClass::Indication,
            method,
            TransactionId::new(rand::random()),
        ))
    }

    /// Converts `Message` to `Indication`.
    ///
    /// # Errors
    ///
    /// If the class of the given message is not `MessageClass::Indication`,
    /// this function will return an `ErrorKind::InvalidInput` error.
    ///
    /// And if the message contains some unknown comprehension-required attributes,
    /// this function will return an `ErrorKind::UnknownAttributes` error.
    pub fn from_message(message: Message<M, A>) -> Result<Self> {
        track_assert_eq!(
            message.class(),
            MessageClass::Indication,
            ErrorKind::InvalidInput
        );
        track!(check_unknown_attributes(&message))?;
        Ok(Indication(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> &M {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> &TransactionId {
        self.0.transaction_id()
    }

    /// Returns an iterator that iterates over the known attributes in the message.
    pub fn attributes(&self) -> impl Iterator<Item = &A> {
        self.0.attributes()
    }

    /// Adds the given attribute to the tail of the attributes in the message.
    pub fn push_attribute(&mut self, attribute: A) {
        self.0.push_attribute(attribute);
    }

    /// Takes ownership of this instance, and returns the internal message.
    pub fn into_message(self) -> Message<M, A> {
        self.0
    }
}
impl<M: Method, A: Attribute> AsRef<Message<M, A>> for Indication<M, A> {
    fn as_ref(&self) -> &Message<M, A> {
        &self.0
    }
}
impl<M: Method, A: Attribute> AsMut<Message<M, A>> for Indication<M, A> {
    fn as_mut(&mut self) -> &mut Message<M, A> {
        &mut self.0
    }
}

/// Success response message.
#[derive(Debug, Clone)]
pub struct SuccessResponse<M, A>(Message<M, A>);
impl<M: Method, A: Attribute> SuccessResponse<M, A> {
    /// Makes a new `SuccessResponse` instance for the success response to the given request.
    pub fn new(request: Request<M, A>) -> Self {
        SuccessResponse(Message::new(
            MessageClass::SuccessResponse,
            request.method().clone(),
            request.transaction_id().clone(),
        ))
    }

    /// Converts `Message` to `SuccessResponse`.
    ///
    /// # Errors
    ///
    /// If the class of the given message is not `MessageClass::SuccessResponse`,
    /// this function will return an `ErrorKind::InvalidInput` error.
    ///
    /// And if the message contains some unknown comprehension-required attributes,
    /// this function will return an `ErrorKind::UnknownAttributes` error.
    pub fn from_message(message: Message<M, A>) -> Result<Self> {
        track_assert_eq!(
            message.class(),
            MessageClass::SuccessResponse,
            ErrorKind::InvalidInput
        );
        track!(check_unknown_attributes(&message))?;
        Ok(SuccessResponse(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> &M {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> &TransactionId {
        self.0.transaction_id()
    }

    /// Returns an iterator that iterates over the known attributes in the message.
    pub fn attributes(&self) -> impl Iterator<Item = &A> {
        self.0.attributes()
    }

    /// Adds the given attribute to the tail of the attributes in the message.
    pub fn push_attribute(&mut self, attribute: A) {
        self.0.push_attribute(attribute);
    }

    /// Takes ownership of this instance, and returns the internal message.
    pub fn into_message(self) -> Message<M, A> {
        self.0
    }
}
impl<M: Method, A: Attribute> AsRef<Message<M, A>> for SuccessResponse<M, A> {
    fn as_ref(&self) -> &Message<M, A> {
        &self.0
    }
}
impl<M: Method, A: Attribute> AsMut<Message<M, A>> for SuccessResponse<M, A> {
    fn as_mut(&mut self) -> &mut Message<M, A> {
        &mut self.0
    }
}

/// Error response message.
#[derive(Debug, Clone)]
pub struct ErrorResponse<M, A>(Message<M, A>);
impl<M: Method, A: Attribute> ErrorResponse<M, A> {
    /// Makes a new `ErrorResponse` instance for the error response to the given request.
    pub fn new(request: Request<M, A>, error: ErrorCode) -> Self
    where
        A: From<ErrorCode>,
    {
        let mut message = Message::new(
            MessageClass::ErrorResponse,
            request.method().clone(),
            request.transaction_id().clone(),
        );
        message.push_attribute(error.into());
        ErrorResponse(message)
    }

    /// Converts `Message` to `ErrorResponse`.
    ///
    /// # Errors
    ///
    /// If the class of the given message is not `MessageClass::ErrorResponse` or
    /// the message does not contains an `ErrorCode` attribute,
    /// this function will return an `ErrorKind::InvalidInput` error.
    ///
    /// And if the message contains some unknown comprehension-required attributes,
    /// this function will return an `ErrorKind::UnknownAttributes` error.
    pub fn from_message(message: Message<M, A>) -> Result<Self> {
        track_assert_eq!(
            message.class(),
            MessageClass::ErrorResponse,
            ErrorKind::InvalidInput
        );
        track!(check_unknown_attributes(&message))?;

        let contains_error_code = message
            .attributes()
            .map(|a| a.get_type())
            .chain(message.unknown_attributes().map(|a| a.get_type()))
            .find(|t| t.as_u16() == ErrorCode::CODEPOINT)
            .is_some();
        track_assert!(contains_error_code, ErrorKind::InvalidInput);
        Ok(ErrorResponse(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> &M {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> &TransactionId {
        self.0.transaction_id()
    }

    /// Returns an iterator that iterates over the known attributes in the message.
    pub fn attributes(&self) -> impl Iterator<Item = &A> {
        self.0.attributes()
    }

    /// Adds the given attribute to the tail of the attributes in the message.
    pub fn push_attribute(&mut self, attribute: A) {
        self.0.push_attribute(attribute);
    }

    /// Takes ownership of this instance, and returns the internal message.
    pub fn into_message(self) -> Message<M, A> {
        self.0
    }
}
impl<M: Method, A: Attribute> AsRef<Message<M, A>> for ErrorResponse<M, A> {
    fn as_ref(&self) -> &Message<M, A> {
        &self.0
    }
}
impl<M: Method, A: Attribute> AsMut<Message<M, A>> for ErrorResponse<M, A> {
    fn as_mut(&mut self) -> &mut Message<M, A> {
        &mut self.0
    }
}

fn check_unknown_attributes<M: Method, A: Attribute>(message: &Message<M, A>) -> Result<()> {
    let required_unknowns = message
        .unknown_attributes()
        .filter_map(|a| {
            if a.get_type().is_comprehension_required() {
                Some(a.get_type())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    track_assert!(
        required_unknowns.is_empty(),
        ErrorKind::UnknownAttributes(required_unknowns)
    );
    Ok(())
}
