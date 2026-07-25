use anyhow::Error as AnyhowError;
use cobalt_format::world::error::WorldLoadingError;
use thiserror::Error;

pub type CobaltResult<T = ()> = Result<T, CobaltServerError>;

#[derive(Debug, Error)]
pub enum CobaltServerError {
    #[error(transparent)]
    WorldLoading(#[from] WorldLoadingError),

    #[error(transparent)]
    Anyhow(#[from] AnyhowError),
}

