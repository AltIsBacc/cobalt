use std::io;

pub type CobaltNetworkResult<T = ()> = Result<T, CobaltNetworkError>;

#[derive(Debug, Error)]
pub enum CobaltNetworkError {
    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("Failed to join the server. Is the server full?")]
    FailedToJoin,
}

