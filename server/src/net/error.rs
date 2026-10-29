use std::io;

use cobalt_protocol::{MessageType, ProtocolError};
use thiserror::Error;

pub(crate) type SessionResult<T = ()> = Result<T, SessionError>;

#[derive(Debug, Error)]
pub(crate) enum SessionError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error("unexpected {0:?} packet")]
    UnexpectedPacket(MessageType),
}
