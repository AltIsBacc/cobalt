use std::{fs::File, io::Read, ops::Deref, path::Path};

use derive_more::derive::{Deref, DerefMut};

use crate::{common::{Loader, codec::BinaryDecode, metadata::SaveMetadata}, io::{BinaryReader, reader::BinaryCursor}, world::{World, error::WorldLoadingError, frame_important::WorldFrameImportant, header::WorldHeader, sections::WorldSections, tiles::{WorldTiles, WorldTilesDecodeContext}}};

#[derive(Deref, DerefMut)]
pub struct WorldLoader<P: AsRef<Path>>(pub P);

impl<P: AsRef<Path>> Loader for WorldLoader<P> {
    type Output = World;
    type Error = WorldLoadingError;

    fn load(&self) -> Result<Self::Output, Self::Error> {
        let mut file = File::open(self.deref())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let mut reader = BinaryCursor::from(&buffer);

        let metadata = SaveMetadata::decode(None, &mut reader)?;
        let sections = WorldSections::decode(None, &mut reader)?;
        let frame_important = WorldFrameImportant::decode(None, &mut reader)?;

        reader.seek(sections.header() as usize); // skip to header
        let header = WorldHeader::decode(None, &mut reader)?;

        reader.seek(sections.tiles() as usize); // skip to tiles
        let tiles = WorldTiles::decode(
            Some(&WorldTilesDecodeContext::new(
                    header.size(), &frame_important
                )),
            &mut reader
        )?;

        Ok(World {
            metadata,
            sections,
            frame_important,
            header,
            tiles
        })
    }
}

