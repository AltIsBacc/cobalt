use num_enum::TryFromPrimitiveError;
use strum::ParseError;
use thiserror::Error;

use crate::{common::metadata::{kinds::SaveKind, versions::SaveVersion}, io::error::{BinaryReadError, BinaryWriteError}};

#[derive(Debug, Error)]
pub enum SaveMetadataError {
    #[error("Unknown version encountered: {0}")]
    UnknownVersion(#[from] TryFromPrimitiveError<SaveVersion>),

    #[error("Invalid signature encountered: {0}")]
    InvalidSignature(ParseError),

    #[error("Unknown file kind encountered: {0}")]
    UnknownKind(#[from] TryFromPrimitiveError<SaveKind>),

    #[error(transparent)]
    BinaryRead(#[from] BinaryReadError),
}

