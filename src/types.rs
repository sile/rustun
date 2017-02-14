#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct U12(u16);
impl U12 {
    pub fn from_u8(value: u8) -> Self {
        U12(value as u16)
    }
    pub fn from_u16(value: u16) -> Option<Self> {
        if value < 0x1000 {
            Some(U12(value))
        } else {
            None
        }
    }
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

pub type TransactionId = [u8; 12];

// TODO: delete(?)
#[derive(Debug)]
pub struct Unused(());
