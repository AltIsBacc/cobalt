use std::io;

use thiserror::Error;

pub type NetResult<T = ()> = Result<T, NetError>;

#[derive(Debug, Error)]
pub enum NetError {
    #[error(transparent)]
    Io(#[from] io::Error),
}
