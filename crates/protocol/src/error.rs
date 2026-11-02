use std::str::Utf8Error;

use thiserror::Error;

pub type ProtocolResult<T = ()> = Result<T, ProtocolError>;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("frame length {0} is shorter than the frame header")]
    FrameTooShort(usize),

    #[error("packet of {0} bytes does not fit in a frame")]
    PacketTooLarge(usize),

    #[error("unexpected end of packet")]
    UnexpectedEof,

    #[error("string is not valid UTF-8")]
    InvalidString(#[from] Utf8Error),

    #[error("variable-length integer does not fit in 32 bits")]
    VarintOverflow,

    #[error("unknown message type {0}")]
    UnknownMessage(u8),

    #[error("unknown network text mode {0}")]
    UnknownTextMode(u8),

    #[error("network text is nested too deeply")]
    TextTooDeep,

    #[error("network text has {0} substitutions, the maximum is 255")]
    TooManySubstitutions(usize),
}
