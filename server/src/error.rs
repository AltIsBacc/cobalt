use thiserror::Error;

use crate::client::ClientId;

pub type ServerResult<T = ()> = Result<T, ServerError>;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(&'static str),

    #[error("the server is full")]
    Full,

    #[error("unknown client {0}")]
    UnknownClient(ClientId),

    #[error("the server is no longer running")]
    Stopped,
}
