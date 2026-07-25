use std::io;

use thiserror::Error;

use crate::{common::metadata::error::SaveMetadataError, io::error::BinaryWriteError, world::{header::error::WorldHeaderError, tiles::error::WorldTilesError}};

#[derive(Debug, Error)]
pub enum WorldLoadingError {
    #[error(transparent)]
    Metadata(#[from] SaveMetadataError),

    #[error(transparent)]
    Header(#[from] WorldHeaderError),

    #[error(transparent)]
    Tiles(#[from] WorldTilesError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("failed to decode world {0}")]
    MalformedData(&'static str),
}

#[derive(Debug, Error)]
pub enum WorldSavingError {
    #[error(transparent)]
    Metadata(#[from] SaveMetadataError),

    #[error(transparent)]
    BinaryWrite(#[from] BinaryWriteError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("failed to write world {0}")]
    WriteFailed(&'static str),
}

