use thiserror::Error;

use crate::io::{csharp_datetime::error::DateTimeError, uleb128::error::Uleb128Error};

pub type ReadResult<T = ()> = Result<T, BinaryReadError>;
pub type WriteResult<T = ()> = Result<T, BinaryWriteError>;

#[derive(Debug, Error)]
pub enum BinaryReadError {
    #[error("unexpected EOF at {pos}: wanted {requested} bytes, {available} available")]
    UnexpectedEof { pos: usize, requested: usize, available: usize },

    #[error("offset {pos} + length {length} overflowed")]
    Overflow { pos: usize, length: usize },

    #[error("encountered an invalid string")]
    InvalidString,

    #[error(transparent)]
    MalformedVarint(#[from] Uleb128Error),

    #[error(transparent)]
    MalformedDateTime(#[from] DateTimeError),
}

#[derive(Debug, Error)]
pub enum BinaryWriteError {

}

