use std::{io, str::Utf8Error};

use thiserror::Error;

use crate::metadata::SaveKind;

pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected end of data at {pos}: requested {requested} bytes, {available} available")]
    UnexpectedEof { pos: usize, requested: usize, available: usize },

    #[error("seek to {pos} is past the end of the data ({len} bytes)")]
    SeekOutOfBounds { pos: usize, len: usize },

    #[error("string is not valid UTF-8")]
    InvalidString(#[from] Utf8Error),

    #[error("variable-length integer does not fit in 32 bits")]
    VarintOverflow,

    #[error("unsupported save version {0}")]
    UnsupportedVersion(i32),

    #[error("unknown save signature {0:?}")]
    UnknownSignature([u8; 7]),

    #[error("unknown save kind {0}")]
    UnknownKind(u8),

    #[error("expected a {expected:?} save, found {found:?}")]
    UnexpectedKind { expected: SaveKind, found: SaveKind },

    #[error("expected {expected} section pointers, found {found}")]
    SectionCount { expected: usize, found: usize },

    #[error("section pointer {0} is negative")]
    NegativeSection(i32),

    #[error("world name is {len} bytes long, the maximum is {max}")]
    NameTooLong { len: usize, max: usize },

    #[error("unknown difficulty {0}")]
    UnknownDifficulty(i32),

    #[error("invalid world dimensions {width}x{height}")]
    InvalidSize { width: i32, height: i32 },

    #[error(transparent)]
    Io(#[from] io::Error),
}
