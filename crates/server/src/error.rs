use std::io;

use thiserror::Error;

pub type ServerResult<T = ()> = Result<T, ServerError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServerError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(&'static str),

    #[error("the server is no longer running")]
    Stopped,

    #[error(transparent)]
    Io(#[from] io::Error),
}
