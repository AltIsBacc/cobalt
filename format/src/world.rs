use derive_more::derive::Debug;
use getset::MutGetters;

use crate::{common::{Loader, Saver, metadata::SaveMetadata}, world::{frame_important::WorldFrameImportant, header::WorldHeader, sections::WorldSections, tiles::WorldTiles}};

pub mod loader;
pub mod saver;
pub mod error;

pub mod sections;
pub mod frame_important;
pub mod header;
pub mod tiles;

#[derive(Debug, MutGetters)]
#[getset(get_mut = "pub")]
pub struct World {
    metadata: SaveMetadata,
    sections: WorldSections,

    #[debug(skip)]
    frame_important: WorldFrameImportant,

    header: WorldHeader,

    #[debug(skip)]
    tiles: WorldTiles 
}

impl World {
    pub fn from_loader<L: Loader<Output = Self>>(loader: L) -> Result<Self, L::Error> {
        loader.load()
    }

    pub fn save_with<S: Saver<Input = Self>>(&self, saver: S) -> Result<(), S::Error> {
        saver.save(self)
    }
}

