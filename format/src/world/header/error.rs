use anyhow::Error as AnyhowError;
use num_enum::TryFromPrimitiveError;
use thiserror::Error;

use crate::{io::error::BinaryReadError, world::header::difficulty::Difficulty};

#[derive(Debug, Error)]
pub enum WorldHeaderError {
    #[error(transparent)]
    BinaryRead(#[from] BinaryReadError),

    #[error(transparent)]
    Generic(#[from] AnyhowError),

    #[error("World name too long!")]
    NameTooLong,

    #[error("Invalid difficulty found: {0}")]
    InvalidDifficulty(#[from] TryFromPrimitiveError<Difficulty>),

    #[error("failed to decode header {0}")]
    MalformedData(&'static str),
}

