use {Result, Method, Attribute};
use types::{U12, TransactionId};
use attribute::RawAttribute;
use message2::{Request, Indication, SuccessResponse, ErrorResponse};

#[derive(Debug, Clone)]
pub struct RawMessage {
    class: Class,
    method: U12,
    transaction_id: TransactionId,
    attributes: Vec<RawAttribute>,
}
impl RawMessage {
    pub fn new(class: Class,
               method: U12,
               transaction_id: TransactionId,
               attributes: Vec<RawAttribute>)
               -> Self {
        RawMessage {
            class: class,
            method: method,
            transaction_id: transaction_id,
            attributes: attributes,
        }
    }
    pub fn class(&self) -> Class {
        self.class
    }
    pub fn method(&self) -> U12 {
        self.method
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }
    pub fn try_into_request<M, A>(self) -> Result<Request<M, A>>
        where M: Method,
              A: Attribute
    {
        panic!()
    }
    pub fn try_into_indication<M, A>(self) -> Result<Indication<M, A>>
        where M: Method,
              A: Attribute
    {
        panic!()
    }
    pub fn try_into_success_response<M, A>(self) -> Result<SuccessResponse<M, A>>
        where M: Method,
              A: Attribute
    {
        panic!()
    }
    pub fn try_into_error_response<M, A>(self) -> Result<ErrorResponse<M, A>>
        where M: Method,
              A: Attribute
    {
        panic!()
    }
}
impl<M, A> From<Request<M, A>> for RawMessage
    where M: Method,
          A: Attribute
{
    fn from(f: Request<M, A>) -> Self {
        RawMessage {
            class: Class::Request,
            method: f.method().as_u12(),
            transaction_id: *f.transaction_id(),
            attributes: Vec::new(), // TODO
        }
    }
}
impl<M, A> From<Indication<M, A>> for RawMessage
    where M: Method,
          A: Attribute
{
    fn from(f: Indication<M, A>) -> Self {
        RawMessage {
            class: Class::Indication,
            method: f.method().as_u12(),
            transaction_id: *f.transaction_id(),
            attributes: Vec::new(), // TODO
        }
    }
}
impl<M, A> From<SuccessResponse<M, A>> for RawMessage
    where M: Method,
          A: Attribute
{
    fn from(f: SuccessResponse<M, A>) -> Self {
        RawMessage {
            class: Class::SuccessResponse,
            method: f.method().as_u12(),
            transaction_id: *f.transaction_id(),
            attributes: Vec::new(), // TODO
        }
    }
}
impl<M, A> From<ErrorResponse<M, A>> for RawMessage
    where M: Method,
          A: Attribute
{
    fn from(f: ErrorResponse<M, A>) -> Self {
        RawMessage {
            class: Class::ErrorResponse,
            method: f.method().as_u12(),
            transaction_id: *f.transaction_id(),
            attributes: Vec::new(), // TODO
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Class {
    Request = 0b00,
    Indication = 0b01,
    SuccessResponse = 0b10,
    ErrorResponse = 0b11,
}
impl Class {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Class::Request),
            0b01 => Some(Class::Indication),
            0b10 => Some(Class::SuccessResponse),
            0b11 => Some(Class::ErrorResponse),
            _ => None,
        }
    }
}
