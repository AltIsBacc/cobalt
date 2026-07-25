use crate::io::uleb128::error::Uleb128Result;

pub mod int;
pub mod long;
pub mod error;

pub trait FromUleb128Bytes: Sized {
    fn from_uleb128_bytes(bytes: &[u8]) -> Uleb128Result<(Self, usize)>;
}

pub trait ToUleb128Bytes {
    fn to_uleb128_bytes(&self) -> Vec<u8>;
}

