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

pub use error::{MessageError, MessageErrorKind};

pub type MessageResult<T> = Result<T, MessageError>;

// TODO:
#[derive(Debug)]
pub struct InvalidMessage {
    pub method: Method,
    pub class: MessageClass,
    pub transaction_id: TransactionId,
    pub error: MessageError,
}

/// Response message.
pub type Response<A> = std::result::Result<SuccessResponse<A>, ErrorResponse<A>>;

/// Request message.
#[derive(Debug, Clone)]
pub struct Request<A>(Message<A>);
impl<A: Attribute> Request<A> {
    /// Makes a new request message.
    pub fn new(method: Method) -> Self {
        Request(Message::new(
            MessageClass::Request,
            method,
            TransactionId::new(rand::random()),
        ))
    }

    // TODO: fix doc
    /// Converts `Message` to `Request`.
    ///
    /// # Errors
    ///
    /// If the class of the given message is not `MessageClass::Request`,
    /// this function will return an `ErrorKind::InvalidInput` error.
    ///
    /// And if the message contains some unknown comprehension-required attributes,
    /// this function will return an `ErrorKind::UnknownAttributes` error.
    pub fn from_message(message: Message<A>) -> Result<Self, MessageError> {
        track_assert_eq!(
            message.class(),
            MessageClass::Request,
            MessageErrorKind::UnexpectedClass
        );
        track!(check_unknown_attributes(&message))?;
        Ok(Request(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> Method {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> TransactionId {
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
    pub fn into_message(self) -> Message<A> {
        self.0
    }
}
impl<A: Attribute> AsRef<Message<A>> for Request<A> {
    fn as_ref(&self) -> &Message<A> {
        &self.0
    }
}
impl<A: Attribute> AsMut<Message<A>> for Request<A> {
    fn as_mut(&mut self) -> &mut Message<A> {
        &mut self.0
    }
}

/// Indication message.
#[derive(Debug, Clone)]
pub struct Indication<A>(Message<A>);
impl<A: Attribute> Indication<A> {
    /// Makes a new indication message.
    pub fn new(method: Method) -> Self {
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
    pub fn from_message(message: Message<A>) -> Result<Self, MessageError> {
        track_assert_eq!(
            message.class(),
            MessageClass::Indication,
            MessageErrorKind::UnexpectedClass
        );
        track!(check_unknown_attributes(&message))?;
        Ok(Indication(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> Method {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> TransactionId {
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
    pub fn into_message(self) -> Message<A> {
        self.0
    }
}
impl<A: Attribute> AsRef<Message<A>> for Indication<A> {
    fn as_ref(&self) -> &Message<A> {
        &self.0
    }
}
impl<A: Attribute> AsMut<Message<A>> for Indication<A> {
    fn as_mut(&mut self) -> &mut Message<A> {
        &mut self.0
    }
}

/// Success response message.
#[derive(Debug, Clone)]
pub struct SuccessResponse<A>(Message<A>);
impl<A: Attribute> SuccessResponse<A> {
    /// Makes a new `SuccessResponse` instance for the success response to the given request.
    pub fn new(request: Request<A>) -> Self {
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
    pub fn from_message(message: Message<A>) -> Result<Self, MessageError> {
        track_assert_eq!(
            message.class(),
            MessageClass::SuccessResponse,
            MessageErrorKind::UnexpectedClass
        );
        track!(check_unknown_attributes(&message))?;
        Ok(SuccessResponse(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> Method {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> TransactionId {
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
    pub fn into_message(self) -> Message<A> {
        self.0
    }
}
impl<A: Attribute> AsRef<Message<A>> for SuccessResponse<A> {
    fn as_ref(&self) -> &Message<A> {
        &self.0
    }
}
impl<A: Attribute> AsMut<Message<A>> for SuccessResponse<A> {
    fn as_mut(&mut self) -> &mut Message<A> {
        &mut self.0
    }
}

/// Error response message.
#[derive(Debug, Clone)]
pub struct ErrorResponse<A>(Message<A>);
impl<A: Attribute> ErrorResponse<A> {
    /// Makes a new `ErrorResponse` instance for the error response to the given request.
    pub fn new(request: Request<A>, error: ErrorCode) -> Self
    where
        A: From<ErrorCode>,
    {
        let mut message = Message::new(
            MessageClass::ErrorResponse,
            request.method(),
            request.transaction_id(),
        );
        message.push_attribute(error.into());
        ErrorResponse(message)
    }

    // TODO: from_broken_message

    // /// Makes a new `ErrorResponse` instance for the error response to the given request.
    // pub fn new2(method: Method, transaction_id: TransactionId, error: ErrorCode) -> Self
    // where
    //     A: From<ErrorCode>,
    // {
    //     let mut message = Message::new(MessageClass::ErrorResponse, method(), transaction_id());
    //     message.push_attribute(error.into());
    //     ErrorResponse(message)
    // }

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
    pub fn from_message(message: Message<A>) -> Result<Self, MessageError> {
        track_assert_eq!(
            message.class(),
            MessageClass::ErrorResponse,
            MessageErrorKind::UnexpectedClass
        );
        track!(check_unknown_attributes(&message))?;

        let contains_error_code = message
            .attributes()
            .map(|a| a.get_type())
            .chain(message.unknown_attributes().map(|a| a.get_type()))
            .find(|t| t.as_u16() == ErrorCode::CODEPOINT)
            .is_some();
        track_assert!(contains_error_code, MessageErrorKind::InvalidInput);
        Ok(ErrorResponse(message))
    }

    /// Returns the method of the message.
    pub fn method(&self) -> Method {
        self.0.method()
    }

    /// Returns the transaction ID of the message.
    pub fn transaction_id(&self) -> TransactionId {
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
    pub fn into_message(self) -> Message<A> {
        self.0
    }
}
impl<A: Attribute> AsRef<Message<A>> for ErrorResponse<A> {
    fn as_ref(&self) -> &Message<A> {
        &self.0
    }
}
impl<A: Attribute> AsMut<Message<A>> for ErrorResponse<A> {
    fn as_mut(&mut self) -> &mut Message<A> {
        &mut self.0
    }
}

fn check_unknown_attributes<A: Attribute>(message: &Message<A>) -> Result<(), MessageError> {
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
        MessageErrorKind::UnknownAttributes(required_unknowns)
    );
    Ok(())
}
