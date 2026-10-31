use std::io;

use cobalt_protocol::{C2sMessage, ProtocolError};
use thiserror::Error;

use crate::error::ServerError;

pub(crate) type SessionResult<T = ()> = Result<T, SessionError>;

#[derive(Debug, Error)]
pub(crate) enum SessionError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error(transparent)]
    Server(#[from] ServerError),

    #[error("unexpected {0:?} packet")]
    UnexpectedPacket(C2sMessage),

    #[error("inventory slot {0} is out of range")]
    InvalidSlot(i16),

    #[error("handshake timed out")]
    HandshakeTimeout,
}
