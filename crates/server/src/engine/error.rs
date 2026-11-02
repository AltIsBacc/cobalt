use std::io;

use cobalt_protocol::ProtocolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ClientError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error("could not compress a tile section")]
    Compression(#[from] io::Error),

    #[error("inventory slot {0} is out of range")]
    InvalidSlot(i16),
}
