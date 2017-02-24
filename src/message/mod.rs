use rand;

use {Method, Attribute};
use types::{TransactionId, ErrorCode};
use method::{Requestable, Indicatable};

pub use self::raw::{RawMessage, Class};

mod raw;

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
    pub fn new<T>(method: T) -> Self
        where T: Requestable + Into<M>
    {
        Request {
            method: method.into(),
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }
    pub fn method(&self) -> &M {
        &self.method
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }
    pub fn into_success_response(self) -> SuccessResponse<M, A> {
        SuccessResponse::new(self.method, self.transaction_id)
    }
    pub fn into_error_response<E>(self, error_code: E) -> ErrorResponse<M, A>
        where E: Into<ErrorCode>
    {
        ErrorResponse::new(self.method, self.transaction_id, error_code.into())
    }
}

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
    pub fn new<T>(method: T) -> Self
        where T: Indicatable + Into<M>
    {
        Indication {
            method: method.into(),
            transaction_id: rand::random(),
            attributes: Vec::new(),
        }
    }
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }
    pub fn method(&self) -> &M {
        &self.method
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }
}

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
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }
    pub fn method(&self) -> &M {
        &self.method
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }
}

#[derive(Debug, Clone)]
pub struct ErrorResponse<M, A> {
    method: M,
    transaction_id: TransactionId,
    error_code: ErrorCode,
    attributes: Vec<A>,
}
impl<M, A> ErrorResponse<M, A>
    where M: Method,
          A: Attribute
{
    fn new(method: M, transaction_id: TransactionId, error_code: ErrorCode) -> Self {
        ErrorResponse {
            method: method,
            transaction_id: transaction_id,
            error_code: error_code,
            attributes: Vec::new(),
        }
    }
    pub fn add_attribute<T: Into<A>>(&mut self, attribute: T) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }
    pub fn with_attribute<T: Into<A>>(mut self, attribute: T) -> Self {
        self.add_attribute(attribute);
        self
    }
    pub fn method(&self) -> &M {
        &self.method
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn attributes(&self) -> &[A] {
        &self.attributes
    }
    pub fn error_code(&self) -> &ErrorCode {
        &self.error_code
    }
}
